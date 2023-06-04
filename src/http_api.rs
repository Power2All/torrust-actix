use actix_cors::Cors;
use actix_remote_ip::RemoteIP;
use actix_web::{App, Error, http, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::error::{InternalError, JsonPayloadError};
use actix_web::http::header::ContentType;
use actix_web::web::ServiceConfig;
use include_dir::{Dir, include_dir};
use log::info;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use scc::ebr::Arc;
use serde::{Serialize, Deserialize};
use serde_json::json;
use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;

use crate::common::{InfoHash, AnnounceEvent};
use crate::config::Configuration;
use crate::tracker::TorrentTracker;
use crate::tracker_objects::stats::StatsEvent;
use crate::tracker_objects::torrents::GetTorrentApi;

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/webgui");

pub fn http_api_cors() -> Cors
{
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET", "POST", "DELETE", "PATCH"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

pub fn http_api_routes(data: Arc<TorrentTracker>) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(web::Data::new(data.clone()));
        cfg.app_data(web::JsonConfig::default().error_handler(|err: JsonPayloadError, _| Error::from(InternalError::from_response(err, HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "json parse error"}))))));
        cfg.service(web::resource("webgui/{path:.*}").route(web::get().to(http_api_static_path)));
        cfg.service(web::resource("api/stats").route(web::get().to(http_api_stats_get)));
        cfg.service(web::resource("api/torrent/{info_hash}").route(web::get().to(http_api_torrent_get)).route(web::delete().to(http_api_torrent_delete)));
        cfg.service(web::resource("api/torrents").route(web::get().to(http_api_torrents_get)));
        cfg.service(web::resource("api/whitelist").route(web::get().to(http_api_whitelist_get_all)));
        cfg.service(web::resource("api/whitelist/reload").route(web::get().to(http_api_whitelist_reload)));
        cfg.service(web::resource("api/whitelist/{info_hash}").route(web::get().to(http_api_whitelist_get)).route(web::post().to(http_api_whitelist_post)).route(web::delete().to(http_api_whitelist_delete)));
        cfg.service(web::resource("api/blacklist").route(web::get().to(http_api_blacklist_get_all)));
        cfg.service(web::resource("api/blacklist/reload").route(web::get().to(http_api_blacklist_reload)));
        cfg.service(web::resource("api/blacklist/{info_hash}").route(web::get().to(http_api_blacklist_get)).route(web::post().to(http_api_blacklist_post)).route(web::delete().to(http_api_blacklist_delete)));
        cfg.service(web::resource("api/keys").route(web::get().to(http_api_keys_get_all)));
        cfg.service(web::resource("api/keys/reload").route(web::get().to(http_api_keys_reload)));
        cfg.service(web::resource("api/keys/{key}").route(web::get().to(http_api_keys_get)).route(web::delete().to(http_api_keys_delete)));
        cfg.service(web::resource("api/keys/{key}/{seconds_valid}").route(web::post().to(http_api_keys_post)).route(web::patch().to(http_api_keys_patch)));
        cfg.service(web::resource("api/maintenance/enable").route(web::get().to(http_api_maintenance_enable)));
        cfg.service(web::resource("api/maintenance/disable").route(web::get().to(http_api_maintenance_disable)));
        cfg.default_service(web::route().to(http_api_not_found));
    })
}

pub async fn http_api(addr: SocketAddr, data: Arc<TorrentTracker>) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    info!("[API] Starting server listener on {}", addr);
    let data_cloned = data;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(http_api_cors())
            .configure(http_api_routes(data_cloned.clone()))
    })
        .keep_alive(Duration::from_secs(900))
        .client_request_timeout(Duration::from_secs(15))
        .client_disconnect_timeout(Duration::from_secs(15))
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();
    let handle = server.handle();
    (handle, server)
}

pub async fn https_api(addr: SocketAddr, data: Arc<TorrentTracker>, ssl_key: String, ssl_cert: String) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    info!("[API] Starting server listener with SSL on {}", addr);
    let data_cloned = data;

    let config = https_api_config(ssl_key, ssl_cert);

    let server = HttpServer::new(move || {
        App::new()
            .wrap(http_api_cors())
            .configure(http_api_routes(data_cloned.clone()))
    })
        .keep_alive(Duration::from_secs(900))
        .client_request_timeout(Duration::from_secs(15))
        .client_disconnect_timeout(Duration::from_secs(15))
        .bind_rustls((addr.ip(), addr.port()), config)
        .unwrap()
        .disable_signals()
        .run();
    let handle = server.handle();
    (handle, server)
}

fn https_api_config(ssl_key: String, ssl_cert: String) -> ServerConfig {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(File::open(ssl_cert).unwrap());
    let key_file = &mut BufReader::new(File::open(ssl_key).unwrap());

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    config.with_single_cert(cert_chain, keys.remove(0)).unwrap()
}

async fn http_api_static_path(path: web::Path<String>) -> HttpResponse
{
    let clean_path = path.into_inner();
    let mut filename = clean_path.trim_start_matches('/');
    if filename.is_empty() {
        filename = "index.htm";
    }
    let mime_type = mime_guess::from_path(filename).first_or_text_plain();

    match STATIC_DIR.get_file(filename) {
        None => {
            HttpResponse::NotFound()
                .content_type(mime_type.to_string())
                .body(STATIC_DIR.get_file("404.htm").unwrap().contents())
        }
        Some(file) => {
            HttpResponse::Ok()
                .body(file.contents())
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct HttpApiTokenCheck {
    token: Option<String>,
}

pub async fn http_api_stats_get(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    HttpResponse::Ok().content_type(ContentType::json()).json(data.get_stats().await)
}

pub async fn http_api_torrent_get(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    if let Some(data_request) = data.get_torrent(info_hash_decoded).await {
        let mut return_data = GetTorrentApi {
            info_hash: info_hash_decoded.to_string(),
            completed: data_request.completed,
            seeders: data_request.seeders,
            leechers: data_request.leechers,
            peers: vec![],
        };
        let mut peer_block = vec![];
        for (peer_id, torrent_peer) in data_request.peers.iter() {
            peer_block.push(json!([
                {
                    "id": peer_id.to_string(),
                    "client": "".to_string()
                },
                {
                    "ip": torrent_peer.peer_addr.to_string(),
                    "updated": torrent_peer.updated.elapsed().as_secs() as i64,
                    "uploaded": torrent_peer.uploaded.0,
                    "downloaded": torrent_peer.downloaded.0,
                    "left": torrent_peer.left.0,
                    "event": match torrent_peer.event {
                        AnnounceEvent::Started => { "Started".to_string() }
                        AnnounceEvent::Stopped => { "Stopped".to_string() }
                        AnnounceEvent::Completed => { "Completed".to_string() }
                        AnnounceEvent::None => { "None".to_string() }
                    }
                }
            ]));
        }
        return_data.peers = peer_block;
        return HttpResponse::Ok().content_type(ContentType::json()).json(json!(&return_data));
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"}))
}

pub async fn http_api_torrent_delete(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.remove_torrent(info_hash_decoded, data.config.persistence).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_torrents_get(request: HttpRequest, remote_ip: RemoteIP, body: web::Json<Vec<String>>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash vector
    let mut torrents = vec![];
    for hash in body.iter() {
        if hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }
        let hash_decoded = match http_api_decode_hex_hash(hash.to_string()).await {
            Ok(data_returned) => { data_returned }
            Err(data_returned) => { return data_returned; }
        };
        if let Some(data_request) = data.get_torrent(hash_decoded).await {
            torrents.push(json!({
                "info_hash": hash_decoded.to_string(),
                "completed": data_request.completed.clone(),
                "seeders": data_request.seeders.clone(),
                "leechers": data_request.leechers.clone(),
            }));
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!(&torrents))
}

pub async fn http_api_whitelist_get_all(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    let whitelist = data.get_whitelist().await;

    return HttpResponse::Ok().content_type(ContentType::json()).json(&whitelist);
}

pub async fn http_api_whitelist_reload(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    data.clear_whitelist().await;
    data.load_whitelists(data.as_ref().clone()).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_whitelist_get(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    if data.check_whitelist(info_hash_decoded).await {
        return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}));
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "not found"}))
}

pub async fn http_api_whitelist_post(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.add_whitelist(info_hash_decoded, false).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_whitelist_delete(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.remove_whitelist(info_hash_decoded).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_blacklist_get_all(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    let blacklist = data.get_blacklist().await;

    return HttpResponse::Ok().content_type(ContentType::json()).json(&blacklist);
}

pub async fn http_api_blacklist_reload(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    data.clear_blacklist().await;
    data.load_blacklists(data.as_ref().clone()).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_blacklist_get(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    if data.check_blacklist(info_hash_decoded).await {
        return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}));
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "not found"}))
}

pub async fn http_api_blacklist_post(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.add_blacklist(info_hash_decoded, false).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_blacklist_delete(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate info_hash
    let info_hash = path.into_inner();
    if info_hash.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid info_hash size (HEX 40 characters)"})); }

    // Decode info_hash into a InfoHash string or give error
    let info_hash_decoded = match http_api_decode_hex_hash(info_hash).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.remove_blacklist(info_hash_decoded).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_keys_get_all(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    let keys = data.get_keys().await;
    return HttpResponse::Ok().content_type(ContentType::json()).json(&keys);
}

pub async fn http_api_keys_reload(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    data.clear_keys().await;
    data.load_keys(data.as_ref().clone()).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_keys_get(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate key
    let key = path.into_inner();
    if key.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid key size (HEX 40 characters)"})); }

    // Decode key into a InfoHash string or give error
    let key_decoded = match http_api_decode_hex_hash(key).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    if data.check_key(key_decoded).await {
        return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}));
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "not found"}))
}

pub async fn http_api_keys_post(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<(String, i64)>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate key
    let (key, valid) = path.into_inner();
    if key.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid key size (HEX 40 characters)"})); }
    if valid < 0 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid seconds_valid, should be 0 or higher"})); }

    // Decode key into a InfoHash string or give error
    let key_decoded = match http_api_decode_hex_hash(key).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.add_key(key_decoded, valid).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_keys_patch(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<(String, i64)>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate key and seconds_valid
    let (key, valid) = path.into_inner();
    if key.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid key size (HEX 40 characters)"})); }
    if valid < 0 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid seconds_valid, should be 0 or higher"})); }

    // Decode key into a InfoHash string or give error
    let key_decoded = match http_api_decode_hex_hash(key).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.remove_key(key_decoded).await;
    data.add_key(key_decoded, valid).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_keys_delete(request: HttpRequest, remote_ip: RemoteIP, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    // Validate key
    let key = path.into_inner();
    if key.len() != 40 { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid key size (HEX 40 characters)"})); }

    // Decode key into a InfoHash string or give error
    let key_decoded = match http_api_decode_hex_hash(key).await {
        Ok(data_returned) => { data_returned }
        Err(data_returned) => { return data_returned; }
    };

    data.remove_key(key_decoded).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_maintenance_enable(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    data.set_stats(StatsEvent::MaintenanceMode, 1).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

pub async fn http_api_maintenance_disable(request: HttpRequest, remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    // Validate token
    let params = web::Query::<HttpApiTokenCheck>::from_query(request.query_string()).unwrap();
    if let Some(response) = http_api_token(params.token.clone(), data.config.clone()).await { return response; }

    data.set_stats(StatsEvent::MaintenanceMode, 0).await;

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}

async fn http_api_not_found(remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_api_stats_log(remote_ip.0, data.clone()).await;

    let filename = "404.htm";
    let mime_type = mime_guess::from_path(filename).first_or_text_plain();

    match STATIC_DIR.get_file(filename) {
        None => { HttpResponse::NotFound().body("") }
        Some(file) => { HttpResponse::NotFound().content_type(mime_type.to_string()).body(file.contents()) }
    }
}

pub async fn http_api_stats_log(ip: IpAddr, tracker: web::Data<Arc<TorrentTracker>>)
{
    if ip.is_ipv4() {
        tracker.update_stats(StatsEvent::Tcp4ConnectionsHandled, 1).await;
        tracker.update_stats(StatsEvent::Tcp4ApiHandled, 1).await;
    } else {
        tracker.update_stats(StatsEvent::Tcp6ConnectionsHandled, 1).await;
        tracker.update_stats(StatsEvent::Tcp6ApiHandled, 1).await;
    }
}

pub async fn http_api_token(token: Option<String>, config: Arc<Configuration>) -> Option<HttpResponse>
{
    match token {
        None => { return Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "missing token"}))); }
        Some(token_code) => {
            if token_code != config.api_key { return Some(HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "invalid token"}))); }
            None
        }
    }
}

pub async fn http_api_decode_hex_hash(hash: String) -> Result<InfoHash, HttpResponse>
{
    return match hex::decode(hash) {
        Ok(hash_result) => {
            Ok(InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()))
        }
        Err(_) => { return Err(HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "unable to decode hex string"}))); }
    };
}