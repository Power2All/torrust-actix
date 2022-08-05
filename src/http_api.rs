use std::collections::HashMap;
use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use axum::{Extension, Router};
use axum::http::{HeaderMap, HeaderValue, StatusCode};
use axum::http::header::HeaderName;
use axum_client_ip::ClientIp;
use axum::routing::{get, post, delete};
use axum_server::{Handle, Server};
use axum_server::tls_rustls::RustlsConfig;
use log::info;
use scc::ebr::Arc;
use scc::HashIndex;
use serde_json::json;
use crate::common::{AnnounceEvent, InfoHash, parse_query};
use crate::config::Configuration;
use crate::tracker::{GetTorrentApi, StatsEvent, TorrentTracker};

pub async fn http_api(handle: Handle, addr: SocketAddr, data: Arc<TorrentTracker>) -> impl Future<Output = Result<(), std::io::Error>>
{
    info!("[API] Starting server listener on {}", addr);
    Server::bind(addr)
        .handle(handle)
        .serve(Router::new()
            .route("/api/stats", get(http_api_stats_get))
            .route("/api/torrents", get(http_api_torrents_get))
            .route("/api/torrent/:info_hash", get(http_api_torrent_get))
            .route("/api/whitelist/:info_hash", get(http_api_whitelist_get).post(http_api_whitelist_post).delete(http_api_whitelist_delete))
            .route("/api/blacklist/:info_hash", get(http_api_blacklist_get).post(http_api_blacklist_post).delete(http_api_blacklist_delete))
            .route("/api/key/:seconds_valid", post(http_api_key_post).delete(http_api_key_delete))
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
            .route("/api/stats", get(http_api_stats_get))
            .route("/api/torrents", get(http_api_torrents_get))
            .route("/api/torrent/:info_hash", get(http_api_torrent_get))
            .route("/api/whitelist/:info_hash", get(http_api_whitelist_get).post(http_api_whitelist_post).delete(http_api_whitelist_delete))
            .route("/api/blacklist/:info_hash", get(http_api_blacklist_get).post(http_api_blacklist_post).delete(http_api_blacklist_delete))
            .route("/api/key/:seconds_valid", post(http_api_key_post).delete(http_api_key_delete))
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
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::OK, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_torrents_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let torrents = state.get_torrents_api().await;
    return (StatusCode::OK, headers, serde_json::to_string(&torrents).unwrap());
}

pub async fn http_api_torrent_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, axum::extract::Path(path_params): axum::extract::Path<HashMap<String, String>>, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map.clone()).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let info_hash: InfoHash = match path_params.get("info_hash") {
        None => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "unknown info_hash");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
        Some(result) => {
            let infohash_decoded = hex::decode(result).unwrap();
            let infohash = <[u8; 20]>::try_from(infohash_decoded[0 .. 20].as_ref()).unwrap();
            InfoHash(infohash)
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
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
}

pub async fn http_api_whitelist_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_whitelist_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_whitelist_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_blacklist_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_blacklist_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_blacklist_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_key_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
}

pub async fn http_api_key_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> (StatusCode, HeaderMap, String)
{
    http_api_stats_log(ip, state.clone()).await;

    let mut headers = HeaderMap::new();
    headers.insert(HeaderName::from_static("content-type"), HeaderValue::from_static("text/plain"));

    let query_map_result = parse_query(params);
    let query_map: HashIndex<String, Vec<Vec<u8>>> = match query_map_result {
        Ok(e) => {
            e
        }
        Err(e) => {
            let mut return_data: HashMap<&str, &str> = HashMap::new();
            return_data.insert("status", "error");
            return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
        }
    };

    if !validate_api_token(state.clone().config.clone(), ip, query_map).await {
        let mut return_data: HashMap<&str, &str> = HashMap::new();
        return_data.insert("status", "invalid token");
        return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&return_data).unwrap());
    }

    let stats = state.get_stats().await;
    return (StatusCode::BAD_REQUEST, headers, serde_json::to_string(&stats).unwrap());
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