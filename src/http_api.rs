use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use axum::{body, Extension, Router};
use axum::body::{Empty, Full};
use axum::extract::Path;
use axum::http::{header, HeaderMap, HeaderValue, Method, StatusCode};
use axum::http::header::HeaderName;
use axum::response::{IntoResponse, Response};
use axum_client_ip::ClientIp;
use axum::routing::{get, post};
use axum_server::{Handle, Server};
use axum_server::tls_rustls::RustlsConfig;
use include_dir::{include_dir, Dir};
use log::info;
use scc::ebr::Arc;
use scc::HashIndex;
use serde_json::json;
use tower_http::cors::{Any, CorsLayer};
use crate::common::{AnnounceEvent, CustomError, InfoHash, parse_query};
use crate::config::Configuration;
use crate::tracker::{GetTorrentApi, StatsEvent, TorrentTracker};

static STATIC_DIR: Dir<'_> = include_dir!("$CARGO_MANIFEST_DIR/webgui");

pub async fn http_api(handle: Handle, addr: SocketAddr, data: Arc<TorrentTracker>) -> impl Future<Output = Result<(), std::io::Error>>
{
    info!("[API] Starting server listener on {}", addr);
    Server::bind(addr)
        .handle(handle)
        .serve(Router::new()
            .route("/webgui/*path", get(http_api_static_path))
            .route("/api/stats", get(http_api_stats_get))
            .route("/api/torrent/:info_hash", get(http_api_torrent_get).delete(http_api_torrent_delete))
            .route("/api/torrents", get(http_api_torrents_get))
            .route("/api/whitelist", get(http_api_whitelist_get_all))
            .route("/api/whitelist/reload", get(http_api_whitelist_reload))
            .route("/api/whitelist/:info_hash", get(http_api_whitelist_get).post(http_api_whitelist_post).delete(http_api_whitelist_delete))
            .route("/api/blacklist", get(http_api_blacklist_get_all))
            .route("/api/blacklist/reload", get(http_api_blacklist_reload))
            .route("/api/blacklist/:info_hash", get(http_api_blacklist_get).post(http_api_blacklist_post).delete(http_api_blacklist_delete))
            .route("/api/keys", get(http_api_keys_get_all))
            .route("/api/keys/reload", get(http_api_keys_reload))
            .route("/api/keys/:key", get(http_api_keys_get).delete(http_api_keys_delete))
            .route("/api/keys/:key/:seconds_valid", post(http_api_keys_post).patch(http_api_keys_patch))
            .route("/api/maintenance/enable", get(http_api_maintenance_enable))
            .route("/api/maintenance/disable", get(http_api_maintenance_disable))
            .layer(CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
                .allow_origin(Any)
                .allow_headers(vec![header::CONTENT_TYPE])
            )
            .layer(Extension(data))
            .into_make_service_with_connect_info::<SocketAddr>()
        )
}

pub async fn https_api(handle: Handle, addr: SocketAddr, data: Arc<TorrentTracker>, ssl_key: String, ssl_cert: String) -> impl Future<Output = Result<(), std::io::Error>>
{
    let ssl_config = RustlsConfig::from_pem_file(
        ssl_cert.clone(),
        ssl_key.clone()
    ).await.unwrap();

    info!("[API] Starting server listener with SSL on {}", addr);
    axum_server::bind_rustls(addr, ssl_config)
        .handle(handle)
        .serve(Router::new()
            .route("/webgui/*path", get(http_api_static_path))
            .route("/api/stats", get(http_api_stats_get))
            .route("/api/torrent/:info_hash", get(http_api_torrent_get).delete(http_api_torrent_delete))
            .route("/api/torrents", get(http_api_torrents_get))
            .route("/api/whitelist", get(http_api_whitelist_get_all))
            .route("/api/whitelist/reload", get(http_api_whitelist_reload))
            .route("/api/whitelist/:info_hash", get(http_api_whitelist_get).post(http_api_whitelist_post).delete(http_api_whitelist_delete))
            .route("/api/blacklist", get(http_api_blacklist_get_all))
            .route("/api/blacklist/reload", get(http_api_blacklist_reload))
            .route("/api/blacklist/:info_hash", get(http_api_blacklist_get).post(http_api_blacklist_post).delete(http_api_blacklist_delete))
            .route("/api/keys", get(http_api_keys_get_all))
            .route("/api/keys/reload", get(http_api_keys_reload))
            .route("/api/keys/:key", get(http_api_keys_get).delete(http_api_keys_delete))
            .route("/api/keys/:key/:seconds_valid", post(http_api_keys_post).patch(http_api_keys_patch))
            .route("/api/maintenance/enable", get(http_api_maintenance_enable))
            .route("/api/maintenance/disable", get(http_api_maintenance_disable))
            .layer(CorsLayer::new()
                .allow_methods([Method::GET, Method::POST, Method::DELETE, Method::PATCH])
                .allow_origin(Any)
                .allow_headers(vec![header::CONTENT_TYPE])
            )
            .layer(Extension(data))
            .into_make_service_with_connect_info::<SocketAddr>()
        )
}

pub async fn http_api_stats_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let stats = state.get_stats().await;
    (StatusCode::OK, headers, serde_json::to_string(&stats).unwrap())
}

pub async fn http_api_torrents_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>, axum::extract::Json(body): axum::extract::Json<serde_json::Value>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    // Validate each requested variable.
    let mut torrents = vec![];
    if body.is_array() {
        match body.as_array() {
            None => {}
            Some(result) => {
                for hash in result.iter() {
                    let info_hash_decoded = hex::decode(hash.as_str().unwrap()).unwrap();
                    let info_hash: InfoHash = InfoHash(<[u8; 20]>::try_from(info_hash_decoded[0 .. 20].as_ref()).unwrap());
                    let torrent = state.get_torrent(info_hash).await;
                    if torrent.is_some() {
                        torrents.push(json!({
                            "info_hash": info_hash.to_string(),
                            "completed": torrent.clone().unwrap().completed,
                            "seeders": torrent.clone().unwrap().seeders,
                            "leechers": torrent.clone().unwrap().leechers,
                        }));
                    }
                }
                return (StatusCode::OK, headers, serde_json::to_string(&torrents).unwrap());
            }
        }
    } else {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid format1");
        return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
    }

    let mut return_data: HashMap<&str, &str> = HashMap::new();
    return_data.insert("status", "unknown torrent");
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_torrent_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result).unwrap();
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    let torrent = state.get_torrent(info_hash).await;
    if torrent.is_some() {
        let mut return_data = GetTorrentApi{
            info_hash: info_hash.to_string(),
            completed: torrent.clone().unwrap().completed,
            seeders: torrent.clone().unwrap().seeders,
            leechers: torrent.clone().unwrap().leechers,
            peers: vec![]
        };
        let mut peer_block = vec![];
        for (peer_id, torrent_peer) in torrent.unwrap().peers.iter() {
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

        return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
    }

    let mut return_data: HashMap<&str, &str> = HashMap::new();
    return_data.insert("status", "unknown torrent");
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_torrent_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    state.remove_torrent(info_hash, state.config.persistence).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_whitelist_get_all(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let whitelist = state.get_whitelist().await;
    (StatusCode::OK, headers, serde_json::to_string(&whitelist).unwrap())
}

pub async fn http_api_whitelist_reload(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    match check_api_token(state.clone().config.clone(), ip, query_map, headers.clone()).await {
        None => {}
        Some(result) => { return result; }
    }

    state.clear_whitelist().await;
    state.load_whitelists().await;

    let return_data = json!({ "status": "ok" });
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_whitelist_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    if state.check_whitelist(info_hash).await {
        let return_data = json!({ "status": "ok" });
        return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
    }

    let return_data = json!({ "status": "not found"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_whitelist_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    state.add_whitelist(info_hash, false).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_whitelist_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    state.remove_whitelist(info_hash).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_blacklist_get_all(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let blacklist = state.get_blacklist().await;
    (StatusCode::OK, headers, serde_json::to_string(&blacklist).unwrap())
}

pub async fn http_api_blacklist_reload(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    state.clear_blacklist().await;
    state.load_blacklists().await;

    let return_data = json!({ "status": "ok" });
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_blacklist_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    if state.check_blacklist(info_hash).await {
        let return_data = json!({ "status": "ok" });
        return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
    }

    let return_data = json!({ "status": "not found"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_blacklist_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    state.add_blacklist(info_hash).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_blacklist_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let info_hash_decoded = hex::decode(result);
            if info_hash_decoded.is_err() || info_hash_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid info_hash" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let info_hash = <[u8; 20]>::try_from(info_hash_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(info_hash)
        }
    };

    state.remove_blacklist(info_hash).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_keys_get_all(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let keys = state.get_keys().await;
    (StatusCode::OK, headers, serde_json::to_string(&keys).unwrap())
}

pub async fn http_api_keys_reload(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    state.clear_keys().await;
    state.load_keys().await;

    let return_data = json!({ "status": "ok" });
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_keys_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let key: InfoHash = match path_params.get("key") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown hash");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let key_decoded = hex::decode(result);
            if key_decoded.is_err() || key_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid key" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let key = <[u8; 20]>::try_from(key_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(key)
        }
    };

    if state.check_key(key).await {
        let return_data = json!({ "status": "ok" });
        return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
    }

    let return_data = json!({ "status": "not found"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_keys_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let key: InfoHash = match path_params.get("key") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown key");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let key_decoded = hex::decode(result);
            if key_decoded.is_err() || key_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid key" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let key = <[u8; 20]>::try_from(key_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(key)
        }
    };

    let seconds_valid: i64 = match path_params.get("seconds_valid") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown timeout");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            match result.parse::<i64>() {
                Ok(result2) => {
                    result2
                }
                Err(_) => {
                    let mut return_data: HashMap<&str, &str> = HashMap::new();
                    return_data.insert("status", "invalid timeout");
                    return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
                }
            }
        }
    };

    state.add_key(key, seconds_valid).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_keys_patch(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let key: InfoHash = match path_params.get("key") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown key");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let key_decoded = hex::decode(result);
            if key_decoded.is_err() || key_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid key" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let key = <[u8; 20]>::try_from(key_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(key)
        }
    };

    let seconds_valid: i64 = match path_params.get("seconds_valid") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown timeout");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            match result.parse::<i64>() {
                Ok(result2) => {
                    result2
                }
                Err(_) => {
                    let mut return_data: HashMap<&str, &str> = HashMap::new();
                    return_data.insert("status", "invalid timeout");
                    return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
                }
            }
        }
    };

    state.remove_key(key).await;
    state.add_key(key, seconds_valid).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_keys_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Path(path_params): Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    let key: InfoHash = match path_params.get("key") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown key");
            return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let key_decoded = hex::decode(result);
            if key_decoded.is_err() || key_decoded.clone().unwrap().len() != 20 {
                let return_data = json!({ "status": "invalid key" });
                return (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap());
            }
            let key = <[u8; 20]>::try_from(key_decoded.unwrap()[0 .. 20].as_ref()).unwrap();
            InfoHash(key)
        }
    };

    state.remove_key(key).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_maintenance_enable(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    state.clone().set_stats(StatsEvent::MaintenanceMode, 1).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_maintenance_disable(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map = match api_query_hashing(query_map_result, headers.clone()) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let check_token = check_api_token(state.clone().config.clone(), ip, query_map.clone(), headers.clone()).await;
    if check_token.is_some() {
        return check_token.unwrap();
    }

    state.clone().set_stats(StatsEvent::MaintenanceMode, 0).await;

    let return_data = json!({ "status": "ok"});
    (StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap())
}

pub async fn http_api_stats_log(ip: IpAddr, tracker: Arc<TorrentTracker>)
{
    if ip.is_ipv4() {
        tracker.update_stats(StatsEvent::Tcp4ConnectionsHandled, 1).await;
        tracker.update_stats(StatsEvent::Tcp4ApiHandled, 1).await;
    } else {
        tracker.update_stats(StatsEvent::Tcp6ConnectionsHandled, 1).await;
        tracker.update_stats(StatsEvent::Tcp6ApiHandled, 1).await;
    }
}

pub async fn validate_api_token(config: Arc<Configuration>, _remote_addr: IpAddr, query: HashIndex<String, Vec<Vec<u8>>>) -> bool
{
    let token = match query.read("token", |_, v| v.clone()) {
        None => { return false; }
        Some(result) => {
            let token = match String::from_utf8(result[0].to_vec()) { Ok(v) => v, Err(_) => return false };
            match token.parse::<String>() { Ok(v) => v, Err(_) => return false }
        }
    };

    if token != config.api_key {
        return false;
    }

    true
}

async fn http_api_static_path(Path(path): Path<String>) -> impl IntoResponse {
    let mut path = path.trim_start_matches('/');
    if path.is_empty() {
        path = "index.htm";
    }
    let mime_type = mime_guess::from_path(path).first_or_text_plain();

    match STATIC_DIR.get_file(path) {
        None => Response::builder()
            .status(StatusCode::NOT_FOUND)
            .body(body::boxed(Empty::new()))
            .unwrap(),
        Some(file) => Response::builder()
            .status(StatusCode::OK)
            .header(
                header::CONTENT_TYPE,
                HeaderValue::from_str(mime_type.as_ref()).unwrap(),
            )
            .body(body::boxed(Full::from(file.contents())))
            .unwrap(),
    }
}

type ApiQueryHashingOk = HashIndex<String, Vec<Vec<u8>>>;
type ApiQueryHashingErr = (StatusCode, HeaderMap, String);

pub fn api_query_hashing(query_map_result: Result<HashIndex<String, Vec<Vec<u8>>>, CustomError>, headers: HeaderMap) -> Result<ApiQueryHashingOk, ApiQueryHashingErr>
{
    match query_map_result {
        Ok(e) => {
            Ok(e)
        }
        Err(_) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "invalid request");
            Err((StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap()))
        }
    }
}

pub async fn check_api_token(config: Arc<Configuration>, ip: IpAddr, query_map: HashIndex<String, Vec<Vec<u8>>>, headers: HeaderMap) -> Option<(StatusCode, HeaderMap, String)>
{
    if !validate_api_token(config, ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return Some((StatusCode::OK, headers, serde_json::to_string(&return_data).unwrap()));
    }
    None
}
