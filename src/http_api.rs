use std::future::Future;
use std::net::{IpAddr, SocketAddr};
use axum::{Extension, Router};
use axum_client_ip::ClientIp;
use axum::routing::get;
use axum_server::{Handle, Server};
use axum_server::tls_rustls::RustlsConfig;
use log::info;
use scc::ebr::Arc;
use crate::tracker::{StatsEvent, TorrentTracker};

pub fn http_api(handle: Handle, addr: SocketAddr, data: Arc<TorrentTracker>) -> impl Future<Output = Result<(), std::io::Error>>
{
    info!("[API] Starting server listener on {}", addr);
    Server::bind(addr)
        .handle(handle)
        .serve(Router::new()
            .route("/stats", get(http_api_stats))
            .layer(Extension(data))
            .into_make_service_with_connect_info::<SocketAddr>()
        )
}

pub fn https_api(handle: Handle, addr: SocketAddr, data: Arc<TorrentTracker>, ssl_key: String, ssl_cert: String) -> impl Future<Output = Result<(), std::io::Error>>
{
    let ssl_config = RustlsConfig::from_pem_file(
        ssl_cert.clone(),
        ssl_key.clone()
    ).await.unwrap();

    info!("[API] Starting server listener with SSL on {}", addr);
    axum_server::bind_rustls(addr, ssl_config)
        .handle(handle)
        .serve(Router::new()
            .route("/stats", get(http_api_stats))
            .layer(Extension(data))
            .into_make_service_with_connect_info::<SocketAddr>()
        )
}

pub async fn http_api_stats(ClientIp(ip): ClientIp, axum::extract::RawQuery(_params): axum::extract::RawQuery, Extension(state): Extension<Arc<TorrentTracker>>) -> String
{
    http_api_stats_log(ip, state.clone()).await;
    let stats = state.get_stats().await;
    serde_json::to_string(&stats).unwrap()
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