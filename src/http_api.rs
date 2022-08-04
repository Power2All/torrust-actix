use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use axum::{Extension, Router};
use axum_client_ip::ClientIp;
use axum::routing::{get, post, delete};
use axum_server::{Handle, Server};
use axum_server::tls_rustls::RustlsConfig;
use log::info;
use scc::ebr::Arc;
use crate::tracker::{StatsEvent, TorrentTracker};

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

pub async fn http_api_stats_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let stats = state.get_stats().await;
    serde_json::to_string(&stats).unwrap()
}

pub async fn http_api_torrents_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_torrent_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_whitelist_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_whitelist_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_whitelist_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_blacklist_get(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_blacklist_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_blacklist_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_key_post(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
}

pub async fn http_api_key_delete(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let return_data: Vec<i64> = vec![];
    serde_json::to_string(&return_data).unwrap()
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