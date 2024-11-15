use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use actix_cors::Cors;
use actix_web::{App, http, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{BytesMut, Data, ServiceConfig};
use futures_util::StreamExt;
use log::{error, info};
use serde_json::json;
use utoipa_swagger_ui::{Config, SwaggerUi};
use crate::api::api_blacklists::{api_service_blacklist_delete, api_service_blacklist_get, api_service_blacklist_post, api_service_blacklists_delete, api_service_blacklists_get, api_service_blacklists_post};
use crate::api::api_keys::{api_service_key_delete, api_service_key_get, api_service_key_post, api_service_keys_delete, api_service_keys_get, api_service_keys_post};
use crate::api::api_stats::api_service_stats_get;
use crate::api::api_torrents::{api_service_torrent_delete, api_service_torrent_get, api_service_torrent_post, api_service_torrents_delete, api_service_torrents_get, api_service_torrents_post};
use crate::api::api_users::{api_service_user_delete, api_service_user_get, api_service_user_post, api_service_users_delete, api_service_users_get, api_service_users_post};
use crate::api::api_whitelists::{api_service_whitelist_delete, api_service_whitelist_get, api_service_whitelist_post, api_service_whitelists_delete, api_service_whitelists_get, api_service_whitelists_post};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::common::structs::custom_error::CustomError;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::configuration::Configuration;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;

pub fn api_service_cors() -> Cors
{
    // This is not a duplicate, each framework has their own CORS configuration.
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

pub fn api_service_routes(data: Arc<ApiServiceData>) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(data.clone()));
        cfg.default_service(web::route().to(api_service_not_found));
        cfg.service(web::resource("stats")
            .route(web::get().to(api_service_stats_get))
        );

        // Torrents API Routing
        cfg.service(web::resource("api/torrent/{info_hash}")
            .route(web::get().to(api_service_torrent_get))
            .route(web::delete().to(api_service_torrent_delete))
        );
        cfg.service(web::resource("api/torrent/{info_hash}/{completed}").route(web::post().to(api_service_torrent_post)));
        cfg.service(web::resource("api/torrents")
            .route(web::get().to(api_service_torrents_get))
            .route(web::post().to(api_service_torrents_post))
            .route(web::delete().to(api_service_torrents_delete))
        );

        // Whitelist API Routing
        cfg.service(web::resource("api/whitelist/{info_hash}")
            .route(web::get().to(api_service_whitelist_get))
            .route(web::post().to(api_service_whitelist_post))
            .route(web::delete().to(api_service_whitelist_delete))
        );
        cfg.service(web::resource("api/whitelists")
            .route(web::get().to(api_service_whitelists_get))
            .route(web::post().to(api_service_whitelists_post))
            .route(web::delete().to(api_service_whitelists_delete))
        );

        // Blacklist API Routing
        cfg.service(web::resource("api/blacklist/{info_hash}")
            .route(web::get().to(api_service_blacklist_get))
            .route(web::post().to(api_service_blacklist_post))
            .route(web::delete().to(api_service_blacklist_delete))
        );
        cfg.service(web::resource("api/blacklists")
            .route(web::get().to(api_service_blacklists_get))
            .route(web::post().to(api_service_blacklists_post))
            .route(web::delete().to(api_service_blacklists_delete))
        );

        // Keys API Routing
        cfg.service(web::resource("api/key/{key_hash}")
            .route(web::get().to(api_service_key_get))
            .route(web::delete().to(api_service_key_delete))
        );
        cfg.service(web::resource("api/key/{key_hash}/{timeout}")
            .route(web::post().to(api_service_key_post))
        );
        cfg.service(web::resource("api/keys")
            .route(web::get().to(api_service_keys_get))
            .route(web::post().to(api_service_keys_post))
            .route(web::delete().to(api_service_keys_delete))
        );

        // Users API Routing (SHA1 ID/UUID hashed key)
        cfg.service(web::resource("api/user/{id}")
            .route(web::get().to(api_service_user_get))
            .route(web::delete().to(api_service_user_delete))
        );
        cfg.service(web::resource("api/user/{id}/{key}/{uploaded}/{downloaded}/{completed}/{updated}/{active}")
            .route(web::post().to(api_service_user_post))
        );
        cfg.service(web::resource("api/users")
            .route(web::get().to(api_service_users_get))
            .route(web::post().to(api_service_users_post))
            .route(web::delete().to(api_service_users_delete))
        );

        // Swagger UI Routing
        if data.torrent_tracker.config.tracker_config.clone().unwrap().swagger.unwrap_or(false) {
            cfg.service(SwaggerUi::new("/swagger-ui/{_:.*}").config(Config::new(["/api/openapi.json"])));
            cfg.service(web::resource("/api/openapi.json")
                .route(web::get().to(api_service_openapi_json))
            );
        }
    })
}

pub async fn api_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    api_server_object: ApiTrackersConfig
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    let keep_alive = api_server_object.keep_alive.unwrap();
    let request_timeout = api_server_object.request_timeout.unwrap();
    let disconnect_timeout = api_server_object.disconnect_timeout.unwrap();
    let worker_threads = api_server_object.threads.unwrap() as usize;

    if api_server_object.ssl.unwrap() {
        info!("[APIS] Starting server listener with SSL on {}", addr);
        if api_server_object.ssl_key.is_none() || api_server_object.ssl_cert.is_none() {
            error!("[APIS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }

        let key_file = &mut BufReader::new(match File::open(match api_server_object.ssl_key.clone() {
            None => { panic!("[APIS] SSL key not set!"); }
            Some(data) => { data }
        }) {
            Ok(data) => { data }
            Err(data) => { panic!("[APIS] SSL key unreadable: {}", data); }
        });
        let certs_file = &mut BufReader::new(match File::open(match api_server_object.ssl_cert.clone() {
            None => { panic!("[APIS] SSL cert not set!"); }
            Some(data) => { data }
        }) {
            Ok(data) => { data }
            Err(data) => { panic!("[APIS] SSL cert unreadable: {}", data); }
        });

        let tls_certs = match rustls_pemfile::certs(certs_file).collect::<Result<Vec<_>, _>>() {
            Ok(data) => { data }
            Err(data) => { panic!("[APIS] SSL cert couldn't be extracted: {}", data); }
        };
        let tls_key = match rustls_pemfile::pkcs8_private_keys(key_file).next().unwrap() {
            Ok(data) => { data }
            Err(data) => { panic!("[APIS] SSL key couldn't be extracted: {}", data); }
        };

        let tls_config = match rustls::ServerConfig::builder().with_no_client_auth().with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key)) {
            Ok(data) => { data }
            Err(data) => { panic!("[APIS] SSL config couldn't be created: {}", data); }
        };

        let server = HttpServer::new(move || {
            App::new()
                .wrap(sentry_actix::Sentry::new())
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
            .wrap(sentry_actix::Sentry::new())
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
            Some(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                "status": "missing token"
            })))
        }
        Some(token_code) => {
            if token_code != config.tracker_config.clone()?.api_key? {
                return Some(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                    "status": "invalid token"
                })));
            }
            None
        }
    }
}

pub async fn api_service_retrieve_remote_ip(request: &HttpRequest, data: Arc<ApiTrackersConfig>) -> Result<IpAddr, ()>
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

pub async fn api_validate_ip(request: &HttpRequest, data: Data<Arc<ApiServiceData>>) -> Result<IpAddr, HttpResponse>
{
    match api_service_retrieve_remote_ip(request, data.api_trackers_config.clone()).await {
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
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    HttpResponse::NotFound().content_type(ContentType::json()).json(json!({
        "status": "not found"
    }))
}

pub fn api_stat_update(ip: IpAddr, data: Arc<TorrentTracker>, stats_ipv4: StatsEvent, stat_ipv6: StatsEvent, count: i64)
{
    match ip {
        IpAddr::V4(_) => {
            let data_clone = data.clone();
            data_clone.update_stats(stats_ipv4, count);
        }
        IpAddr::V6(_) => {
            let data_clone = data.clone();
            data_clone.update_stats(stat_ipv6, count);
        }
    }
}

pub async fn api_validation(request: &HttpRequest, data: &Data<Arc<ApiServiceData>>) -> Option<HttpResponse>
{
    match api_validate_ip(request, data.clone()).await {
        Ok(ip) => { api_stat_update(ip, data.torrent_tracker.clone(), StatsEvent::Tcp4ApiHandled, StatsEvent::Tcp6ApiHandled, 1); },
        Err(result) => { return Some(result); }
    }
    None
}

pub async fn api_service_openapi_json() -> HttpResponse
{
    let openapi_file = include_str!("../openapi.json");
    HttpResponse::Ok().content_type(ContentType::json()).body(openapi_file)
}

pub async fn api_parse_body(mut payload: web::Payload) -> Result<BytesMut, CustomError>
{
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(data) => { data }
            Err(_) => { return Err(CustomError::new("chunk error")) }
        };

        if (body.len() + chunk.len()) > 1_048_576 {
            return Err(CustomError::new("chunk size exceeded"));
        }

        body.extend_from_slice(&chunk);
    }
    Ok(body)
}