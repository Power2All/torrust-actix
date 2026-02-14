use crate::api::api_blacklists::{
    api_service_blacklist_delete,
    api_service_blacklist_get,
    api_service_blacklist_post,
    api_service_blacklists_delete,
    api_service_blacklists_get,
    api_service_blacklists_post
};
use crate::api::api_certificate::{
    api_service_certificate_reload,
    api_service_certificate_status
};
use crate::api::api_keys::{
    api_service_key_delete,
    api_service_key_get,
    api_service_key_post,
    api_service_keys_delete,
    api_service_keys_get,
    api_service_keys_post
};
use crate::api::api_stats::{
    api_service_prom_get,
    api_service_stats_get
};
use crate::api::api_torrents::{
    api_service_torrent_delete,
    api_service_torrent_get,
    api_service_torrent_post,
    api_service_torrents_delete,
    api_service_torrents_get,
    api_service_torrents_post
};
use crate::api::api_users::{
    api_service_user_delete,
    api_service_user_get,
    api_service_user_post,
    api_service_users_delete,
    api_service_users_get,
    api_service_users_post
};
use crate::api::api_whitelists::{
    api_service_whitelist_delete,
    api_service_whitelist_get,
    api_service_whitelist_post,
    api_service_whitelists_delete,
    api_service_whitelists_get,
    api_service_whitelists_post
};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::common::structs::custom_error::CustomError;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::configuration::Configuration;
use crate::security::security::{
    constant_time_eq,
    validate_remote_ip
};
use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use actix_cors::Cors;
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{
    BytesMut,
    Data,
    ServiceConfig
};
use actix_web::{
    http,
    web,
    App,
    HttpRequest,
    HttpResponse,
    HttpServer
};
use futures_util::StreamExt;
use log::{
    error,
    info
};
use serde_json::json;
use std::future::Future;
use std::net::{
    IpAddr,
    SocketAddr
};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use utoipa_swagger_ui::{
    Config,
    SwaggerUi
};

pub fn api_service_cors() -> Cors
{
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

pub fn api_service_routes(data: Arc<ApiServiceData>) -> Box<dyn Fn(&mut ServiceConfig) + Send + Sync>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(Arc::clone(&data)));
        cfg.default_service(web::route().to(api_service_not_found));
        cfg.service(web::resource("stats").route(web::get().to(api_service_stats_get)));
        cfg.service(web::resource("metrics").route(web::get().to(api_service_prom_get)));
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
        cfg.service(web::resource("api/certificate/reload")
            .route(web::post().to(api_service_certificate_reload))
        );
        cfg.service(web::resource("api/certificate/status")
            .route(web::get().to(api_service_certificate_status))
        );
        if data.torrent_tracker.config.tracker_config.swagger {
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
    let keep_alive = api_server_object.keep_alive;
    let request_timeout = api_server_object.request_timeout;
    let disconnect_timeout = api_server_object.disconnect_timeout;
    let worker_threads = api_server_object.threads as usize;
    let api_service_data = Arc::new(ApiServiceData {
        torrent_tracker: Arc::clone(&data),
        api_trackers_config: Arc::new(api_server_object.clone()),
    });
    let app_factory = move || {
        let cors = api_service_cors();
        let sentry_wrap = sentry_actix::Sentry::new();
        App::new()
            .wrap(cors)
            .wrap(sentry_wrap)
            .configure(api_service_routes(Arc::clone(&api_service_data)))
    };
    if api_server_object.ssl {
        info!("[APIS] Starting server listener with SSL on {addr}");
        if api_server_object.ssl_key.is_empty() || api_server_object.ssl_cert.is_empty() {
            error!("[APIS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }
        let server_id = ServerIdentifier::ApiServer(addr.to_string());
        if let Err(e) = data.certificate_store.load_certificate(
            server_id.clone(),
            &api_server_object.ssl_cert,
            &api_server_object.ssl_key,
        ) {
            panic!("[APIS] Failed to load SSL certificate: {}", e);
        }
        let resolver = match DynamicCertificateResolver::new(
            Arc::clone(&data.certificate_store),
            server_id,
        ) {
            Ok(resolver) => Arc::new(resolver),
            Err(e) => panic!("[APIS] Failed to create certificate resolver: {}", e),
        };
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        let server = HttpServer::new(app_factory)
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
    info!("[API] Starting server listener on {addr}");
    let server = HttpServer::new(app_factory)
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
    let event = if ip.is_ipv4() {
        StatsEvent::Tcp4ConnectionsHandled
    } else {
        StatsEvent::Tcp6ConnectionsHandled
    };
    tracker.update_stats(event, 1);
}

pub async fn api_service_token(token: Option<String>, config: Arc<Configuration>) -> Option<HttpResponse>
{
    let token_code = match token {
        Some(token) => token,
        None => {
            return Some(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                "status": "missing token"
            })));
        }
    };
    if !constant_time_eq(&token_code, &config.tracker_config.api_key) {
        return Some(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
            "status": "invalid token"
        })));
    }
    None
}

pub async fn api_service_retrieve_remote_ip(request: &HttpRequest, data: Arc<ApiTrackersConfig>) -> Result<IpAddr, ()>
{
    let origin_ip = request.peer_addr().map(|addr| addr.ip()).ok_or(())?;
    if !data.trusted_proxies {
        return Ok(origin_ip);
    }
    request.headers()
        .get(&data.real_ip)
        .and_then(|header| header.to_str().ok())
        .and_then(|ip_str| {
            validate_remote_ip(ip_str, data.trusted_proxies).ok()?;
            IpAddr::from_str(ip_str).ok()
        })
        .map(Ok)
        .unwrap_or(Ok(origin_ip))
}

pub async fn api_validate_ip(request: &HttpRequest, data: Data<Arc<ApiServiceData>>) -> Result<IpAddr, HttpResponse>
{
    match api_service_retrieve_remote_ip(request, Arc::clone(&data.api_trackers_config)).await {
        Ok(ip) => {
            api_service_stats_log(ip, Arc::clone(&data.torrent_tracker)).await;
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
    if let Some(error_return) = api_validation(&request, &data).await {
        return error_return;
    }
    HttpResponse::NotFound().content_type(ContentType::json()).json(json!({
        "status": "not found"
    }))
}

pub fn api_stat_update(ip: IpAddr, data: Arc<TorrentTracker>, stats_ipv4: StatsEvent, stat_ipv6: StatsEvent, count: i64)
{
    let event = if ip.is_ipv4() {
        stats_ipv4
    } else {
        stat_ipv6
    };
    data.update_stats(event, count);
}

pub async fn api_validation(request: &HttpRequest, data: &Data<Arc<ApiServiceData>>) -> Option<HttpResponse>
{
    match api_validate_ip(request, data.clone()).await {
        Ok(ip) => {
            api_stat_update(
                ip,
                Arc::clone(&data.torrent_tracker),
                StatsEvent::Tcp4ApiHandled,
                StatsEvent::Tcp6ApiHandled,
                1
            );
            None
        }
        Err(result) => Some(result),
    }
}

pub async fn api_service_openapi_json() -> HttpResponse
{
    let openapi_file = include_str!("../openapi.json");
    HttpResponse::Ok().content_type(ContentType::json()).body(openapi_file)
}

pub async fn api_parse_body(mut payload: web::Payload) -> Result<BytesMut, CustomError>
{
    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|_| CustomError::new("chunk error"))?;

        if body.len() + chunk.len() > 1_048_576 {
            return Err(CustomError::new("chunk size exceeded"));
        }
        body.extend_from_slice(&chunk);
    }
    Ok(body)
}