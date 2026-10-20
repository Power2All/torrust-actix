use crate::api::api_blacklists::{
    api_service_blacklist_clear,
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
    api_service_keys_clear,
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
    api_service_users_clear,
    api_service_users_delete,
    api_service_users_get,
    api_service_users_post
};
use crate::api::api_whitelists::{
    api_service_whitelist_clear,
    api_service_whitelist_delete,
    api_service_whitelist_get,
    api_service_whitelist_post,
    api_service_whitelists_delete,
    api_service_whitelists_get,
    api_service_whitelists_post
};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::HexParseError;
use serde::de::DeserializeOwned;
use std::str::FromStr;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::configuration::Configuration;
use crate::security::security::constant_time_eq;
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
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;
use utoipa_swagger_ui::{
    Config,
    SwaggerUi
};

/// Builds the CORS policy for the management API (any origin, common methods, `Authorization`
/// and `Content-Type` headers allowed).
pub fn api_service_cors() -> Cors
{
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .allowed_header(http::header::AUTHORIZATION)
        .max_age(1)
}

/// Returns the Actix route configuration for the management API: `/stats`, `/metrics`,
/// the `api/torrent(s)`, `api/whitelist(s)`, `api/blacklist(s)`, `api/key(s)`, `api/user(s)`
/// and `api/certificate/*` resources, plus optional Swagger UI.
pub fn api_service_routes(data: Arc<ApiServiceData>) -> Box<dyn Fn(&mut ServiceConfig) + Send + Sync>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(Arc::clone(&data)));
        cfg.default_service(web::route().to(api_service_not_found));
        cfg.service(web::resource("stats")
            .route(web::get().to(api_service_stats_get)));
        cfg.service(web::resource("metrics")
            .route(web::get().to(api_service_prom_get)));
        cfg.service(web::resource("api/torrent/{info_hash}")
            .route(web::get().to(api_service_torrent_get))
            .route(web::delete().to(api_service_torrent_delete))
        );
        cfg.service(web::resource("api/torrent/{info_hash}/{completed}")
            .route(web::post().to(api_service_torrent_post)));
        cfg.service(web::resource("api/torrents")
            .route(web::get().to(api_service_torrents_get))
            .route(web::post().to(api_service_torrents_post))
            .route(web::delete().to(api_service_torrents_delete))
        );
        cfg.service(web::resource("api/whitelist/clear")
            .route(web::delete().to(api_service_whitelist_clear))
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
        cfg.service(web::resource("api/blacklist/clear")
            .route(web::delete().to(api_service_blacklist_clear))
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
        cfg.service(web::resource("api/keys/clear")
            .route(web::delete().to(api_service_keys_clear))
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
        cfg.service(web::resource("api/users/clear")
            .route(web::delete().to(api_service_users_clear))
        );
        cfg.service(web::resource("api/certificate/reload")
            .route(web::post().to(api_service_certificate_reload))
        );
        cfg.service(web::resource("api/certificate/status")
            .route(web::get().to(api_service_certificate_status))
        );
        if data.torrent_tracker.config.tracker_config.swagger {
            cfg.service(SwaggerUi::new("/swagger-ui/{_:.*}")
                .config(Config::new(["/api/openapi.json"])));
            cfg.service(web::resource("/api/openapi.json")
                .route(web::get().to(api_service_openapi_json))
            );
        }
    })
}

/// Starts an API listener (HTTP, or HTTPS when `ssl` is set) on `addr`.
///
/// Returns the Actix [`ServerHandle`] for shutdown plus the server future to await.
///
/// # Panics / exit
///
/// Exits the process when the address cannot be bound or the TLS material is missing.
pub async fn api_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    api_server_object: ApiTrackersConfig
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    let tls_config = api_server_object.ssl.then(|| {
        info!("[APIS] Starting server listener with SSL on {addr}");
        if api_server_object.ssl_key.is_empty() || api_server_object.ssl_cert.is_empty() {
            error!("[APIS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }
        let server_id = ServerIdentifier::ApiServer(addr.to_string());
        DynamicCertificateResolver::server_config(&data.certificate_store, server_id, &api_server_object.ssl_cert, &api_server_object.ssl_key)
            .unwrap_or_else(|e| panic!("[APIS] Failed to load SSL certificate: {e}"))
    });
    let tag = if tls_config.is_some() { "APIS" } else { "API" };
    if tls_config.is_none() {
        info!("[API] Starting server listener on {addr}");
    }
    let api_service_data = Arc::new(ApiServiceData {
        torrent_tracker: Arc::clone(&data),
        api_trackers_config: Arc::new(api_server_object.clone()),
    });
    let server = HttpServer::new(move || {
        App::new()
            .wrap(api_service_cors())
            .wrap(sentry_actix::Sentry::new())
            .configure(api_service_routes(Arc::clone(&api_service_data)))
    })
        .keep_alive(Duration::from_secs(api_server_object.keep_alive))
        .client_request_timeout(Duration::from_secs(api_server_object.request_timeout))
        .client_disconnect_timeout(Duration::from_secs(api_server_object.disconnect_timeout))
        .workers(api_server_object.threads as usize)
        .max_connections(api_server_object.max_connections as usize);
    let server = match tls_config {
        // Caps in-flight TLS handshakes per worker. Writes one process-global in `actix-tls`,
        // which is why every TLS listener here passes the same value.
        Some(tls_config) => server
            .max_connection_rate(data.config.tracker_config.tls_connection_rate as usize)
            .bind_rustls_0_23((addr.ip(), addr.port()), tls_config),
        None => server.bind((addr.ip(), addr.port())),
    }
        .unwrap_or_else(|e| {
            error!("[{tag}] Unable to bind to {addr}: {e}");
            exit(1);
        })
        .disable_signals()
        .run();
    (server.handle(), server)
}

/// Extracts the API token from the `Authorization` header (with or without a `Bearer` prefix),
/// falling back to the legacy `?token=` query parameter.
pub fn api_extract_token(request: &HttpRequest) -> Option<String>
{
    if let Some(value) = request.headers().get(http::header::AUTHORIZATION).and_then(|h| h.to_str().ok()) {
        let token = value
            .strip_prefix("Bearer ")
            .or_else(|| value.strip_prefix("bearer "))
            .unwrap_or(value)
            .trim();
        if !token.is_empty() {
            return Some(token.to_string());
        }
    }
    web::Query::<QueryToken>::from_query(request.query_string())
        .ok()
        .and_then(|params| params.token.clone())
}

/// Validates the request's API token against the configured key using a constant-time comparison.
///
/// Returns `None` when the token is valid, or `Some(response)` with the JSON error to send.
pub async fn api_service_token(request: &HttpRequest, config: Arc<Configuration>) -> Option<HttpResponse>
{
    let token_code = match api_extract_token(request) {
        Some(token) => token,
        None => {
            return Some(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                "status": "missing token"
            })));
        }
    };
    // An empty token never authenticates: with `api_key = ""` configured, `?token=` would
    // otherwise match it and open every endpoint.
    if token_code.is_empty() || !constant_time_eq(&token_code, &config.tracker_config.api_key) {
        return Some(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
            "status": "invalid token"
        })));
    }
    None
}

/// Catch-all handler returning a JSON `unknown request` error with HTTP 404.
pub async fn api_service_not_found(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await {
        return error_return;
    }
    HttpResponse::NotFound().content_type(ContentType::json()).json(json!({
        "status": "not found"
    }))
}

/// The IP check and the token check every authenticated endpoint starts with, in that order;
/// `Some` is the response to send instead of handling the request.
pub async fn api_guard(request: &HttpRequest, data: &Data<Arc<ApiServiceData>>) -> Option<HttpResponse>
{
    if let Some(response) = api_validation(request, data).await {
        return Some(response);
    }
    api_service_token(request, Arc::clone(&data.torrent_tracker.config)).await
}

/// Per-request guard: resolves the client IP and records the connection and API-handled
/// statistics. Token validation is done separately by [`api_service_token`].
///
/// Returns `Some(response)` with the error to send, or `None` when the request may proceed.
pub async fn api_validation(request: &HttpRequest, data: &Data<Arc<ApiServiceData>>) -> Option<HttpResponse>
{
    let config = &data.api_trackers_config;
    let Some(ip) = crate::http::http::retrieve_remote_ip(request, config.trusted_proxies, &config.trusted_proxy_ips, &config.real_ip) else {
        return Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({
            "status": "invalid ip"
        })));
    };
    data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4ConnectionsHandled, StatsEvent::Tcp6ConnectionsHandled, 1);
    data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4ApiHandled, StatsEvent::Tcp6ApiHandled, 1);
    None
}

/// `GET /api/openapi.json` — serves the generated OpenAPI specification for Swagger UI.
///
/// Behind the same token as the rest of the API: the specification enumerates every management
/// endpoint, and there is no reason for it to be the one route that answers anonymously.
pub async fn api_service_openapi_json(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let openapi_file = include_str!("../openapi.json");
    HttpResponse::Ok().content_type(ContentType::json()).body(openapi_file)
}

/// Reads a request body into memory, capped at 1 MiB, and parses it as JSON. Runs inside the
/// handler, after [`api_guard`], so an unauthenticated request is refused before its body is read.
///
/// # Errors
///
/// Returns `400 {"status": ...}` with `chunk error`, `chunk size exceeded` or `bad json body`.
pub async fn api_json_body<T: DeserializeOwned>(mut payload: web::Payload) -> Result<T, Box<HttpResponse>>
{
    let bad = |status: &str| Box::new(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": status})));
    let mut body = BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk.map_err(|_| bad("chunk error"))?;
        if body.len() + chunk.len() > 1_048_576 {
            return Err(bad("chunk size exceeded"));
        }
        body.extend_from_slice(&chunk);
    }
    serde_json::from_slice(&body).map_err(|_| bad("bad json body"))
}

/// Parses a 40-character hex id ([`InfoHash`] or [`UserId`](crate::tracker::structs::user_id::UserId))
/// from a path or body value.
///
/// # Errors
///
/// Returns `400 {"status": "bad <what>"}` for a wrong length and `400 {"status": "invalid <what>"}`
/// for a non-hex character.
pub fn parse_hex_id<T: FromStr<Err = HexParseError>>(value: &str, what: &str) -> Result<T, Box<HttpResponse>>
{
    value.parse().map_err(|e| {
        let status = match e {
            HexParseError::InvalidLength => format!("bad {what}"),
            HexParseError::InvalidCharacter => format!("invalid {what}"),
        };
        Box::new(HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": status})))
    })
}
