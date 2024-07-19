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
use crate::api::structs::query_token::QueryToken;
use crate::config::structs::configuration::Configuration;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub fn api_service_cors() -> Cors
{
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

pub fn api_service_routes(data: Arc<TorrentTracker>) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(data.clone()));
        cfg.default_service(web::route().to(api_service_not_found));
        cfg.service(web::resource("api/stats").route(web::get().to(api_service_stats_get)));
    })
}

pub async fn api_service_stats_get(request: HttpRequest, remote_ip: RemoteIP, data: Data<Arc<TorrentTracker>>) -> HttpResponse
{
    api_service_stats_log(remote_ip.0, data.clone()).await;

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.config.clone()).await { return response; }

    let stats = data.get_stats();
    HttpResponse::Ok().content_type(ContentType::json()).json(stats)
}

pub async fn api_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    keep_alive: u64,
    client_request_timeout: u64,
    client_disconnect_timeout: u64,
    threads: u64,
    ssl: (bool, Option<String>, Option<String>) /* 0: ssl enabled, 1: cert, 2: key */
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    if ssl.0 {
        info!("[API] Starting server listener with SSL on {}", addr);
        if ssl.1.is_none() || ssl.2.is_none() {
            error!("[API] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }

        let certs_file = &mut BufReader::new(File::open(ssl.1.clone().unwrap()).unwrap());
        let key_file = &mut BufReader::new(File::open(ssl.2.clone().unwrap()).unwrap());

        let tls_certs = rustls_pemfile::certs(certs_file)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let tls_key = rustls_pemfile::pkcs8_private_keys(key_file)
            .next()
            .unwrap()
            .unwrap();

        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .unwrap();

        let server = HttpServer::new(move || {
            App::new()
                .wrap(api_service_cors())
                .configure(api_service_routes(data.clone()))
        })
            .keep_alive(Duration::from_secs(keep_alive))
            .client_request_timeout(Duration::from_secs(client_request_timeout))
            .client_disconnect_timeout(Duration::from_secs(client_disconnect_timeout))
            .workers(threads as usize)
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
            .configure(api_service_routes(data.clone()))
    })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(client_request_timeout))
        .client_disconnect_timeout(Duration::from_secs(client_disconnect_timeout))
        .workers(threads as usize)
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();

    (server.handle(), server)
}

pub async fn api_service_stats_log(ip: IpAddr, tracker: Data<Arc<TorrentTracker>>)
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
            return Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                "status": "missing token"
            })));
        }
        Some(token_code) => {
            if token_code != config.api_key {
                return Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                    "status": "invalid token"
                })));
            }
            None
        }
    }
}

pub async fn api_service_retrieve_remote_ip(request: HttpRequest, data: Data<Arc<TorrentTracker>>) -> Result<IpAddr, ()>
{
    let origin_ip = match request.peer_addr() {
        None => {
            return Err(());
        }
        Some(ip) => {
            ip.ip()
        }
    };
    match request.headers().get(data.config.http_real_ip.clone()) {
        Some(header) => {
            if header.to_str().is_ok() {
                return if let Ok(ip) = IpAddr::from_str(header.to_str().unwrap()) {
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

pub async fn api_validate_ip(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> Result<IpAddr, HttpResponse>
{
    return match api_service_retrieve_remote_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            api_service_stats_log(ip, data.clone()).await;
            Ok(ip)
        }
        Err(_) => {
            Err(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                "status": "invalid ip"
            })))
        }
    }
}

pub async fn api_service_not_found(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
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
