use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use actix_cors::Cors;
use actix_remote_ip::RemoteIP;
use actix_web::{App, http, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{Data, ServiceConfig};
use log::{error, info};
use serde_json::json;
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::configuration::Configuration;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub fn api_service_cors() -> Cors
{
    // This is not a duplicate, each framework has their own CORS configuration.
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

pub fn api_service_routes(data: Arc<ApiServiceData>) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(data.clone()));
        cfg.default_service(web::route().to(api_service_not_found));
        cfg.service(web::resource("api/stats").route(web::get().to(api_service_stats_get)));
    })
}

pub async fn api_service_stats_get(request: HttpRequest, remote_ip: RemoteIP, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    api_service_stats_log(remote_ip.0, data.torrent_tracker.clone()).await;

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await {
        return response;
    }

    let stats = data.torrent_tracker.get_stats();
    HttpResponse::Ok().content_type(ContentType::json()).json(stats)
}

pub async fn api_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    api_server_object: ApiTrackersConfig
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    let keep_alive = api_server_object.keep_alive.clone().unwrap();
    let request_timeout = api_server_object.request_timeout.clone().unwrap();
    let disconnect_timeout = api_server_object.disconnect_timeout.clone().unwrap();
    let worker_threads = api_server_object.threads.clone().unwrap() as usize;

    if api_server_object.ssl.unwrap() {
        info!("[APIS] Starting server listener with SSL on {}", addr);
        if api_server_object.ssl_key.is_none() || api_server_object.ssl_cert.is_none() {
            error!("[APIS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }

        let key_file = &mut BufReader::new(File::open(api_server_object.ssl_key.clone().unwrap()).unwrap());
        let certs_file = &mut BufReader::new(File::open(api_server_object.ssl_cert.clone().unwrap()).unwrap());

        let tls_certs = rustls_pemfile::certs(certs_file).collect::<Result<Vec<_>, _>>().unwrap();
        let tls_key = match rustls_pemfile::pkcs8_private_keys(key_file).next().unwrap() {
            Err(_) => { exit(1); }
            Ok(data) => { data }
        };

        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .unwrap();

        let server = HttpServer::new(move || {
            App::new()
                .wrap(api_service_cors())
                .configure(api_service_routes(Arc::new(ApiServiceData {
                    torrent_tracker: data.clone(),
                    api_trackers_config: Arc::new(api_server_object.clone())
                })))
        })
            .keep_alive(Duration::from_secs(keep_alive))
            .client_request_timeout(Duration::from_secs(request_timeout))
            .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
            .workers(worker_threads)
            .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
            .unwrap()
            .disable_signals()
            .run();

        return (server.handle(), server);
    }

    info!("[API] Starting server listener on {}", addr);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(api_service_cors())
            .configure(api_service_routes(Arc::new(ApiServiceData {
                torrent_tracker: data.clone(),
                api_trackers_config: Arc::new(api_server_object.clone())
            })))
    })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(request_timeout))
        .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
        .workers(worker_threads)
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();

    (server.handle(), server)
}

pub async fn api_service_stats_log(ip: IpAddr, tracker: Arc<TorrentTracker>)
{
    if ip.is_ipv4() {
        tracker.update_stats(StatsEvent::Tcp4ConnectionsHandled, 1);
    } else {
        tracker.update_stats(StatsEvent::Tcp6ConnectionsHandled, 1);
    }
}

pub async fn api_service_token(token: Option<String>, config: Arc<Configuration>) -> Option<HttpResponse>
{
    match token {
        None => {
            Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                "status": "missing token"
            })))
        }
        Some(token_code) => {
            if token_code != config.tracker_config.clone()?.api_key? {
                return Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                    "status": "invalid token"
                })));
            }
            None
        }
    }
}

pub async fn api_service_retrieve_remote_ip(request: HttpRequest, data: Arc<ApiTrackersConfig>) -> Result<IpAddr, ()>
{
    let origin_ip = match request.peer_addr() {
        None => {
            return Err(());
        }
        Some(ip) => {
            ip.ip()
        }
    };
    match request.headers().get(data.real_ip.clone().unwrap()) {
        Some(header) => {
            if header.to_str().is_ok() {
                if let Ok(ip) = IpAddr::from_str(header.to_str().unwrap()) {
                    Ok(ip)
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        }
        None => {
            Ok(origin_ip)
        }
    }
}

pub async fn api_validate_ip(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> Result<IpAddr, HttpResponse>
{
    match api_service_retrieve_remote_ip(request.clone(), data.api_trackers_config.clone()).await {
        Ok(ip) => {
            api_service_stats_log(ip, data.torrent_tracker.clone()).await;
            Ok(ip)
        }
        Err(_) => {
            Err(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                "status": "invalid ip"
            })))
        }
    }
}

pub async fn api_service_not_found(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    let _ = match api_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            return result;
        }
    };

    HttpResponse::NotFound().content_type(ContentType::json()).json(json!({
        "status": "not found"
    }))
}
