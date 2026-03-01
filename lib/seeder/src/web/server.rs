use crate::config::structs::torrents_file::TorrentsFile;
use crate::config::structs::web_config::WebConfig;
use crate::stats::shared_stats::SharedStats;
use crate::web::api::{
    add_torrent,
    browse,
    delete_torrent,
    get_config,
    get_index,
    get_logo,
    get_status,
    get_torrents,
    post_login,
    post_logout,
    update_config,
    update_torrent,
};
use crate::web::structs::app_state::{
    AppState,
    SessionStore
};
use actix_web::{
    web::{
        self,
        Data
    },
    App,
    HttpServer,
};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{
    watch,
    Mutex,
    RwLock
};

pub async fn start(
    config: WebConfig,
    yaml_path: PathBuf,
    shared_file: Arc<RwLock<TorrentsFile>>,
    stats: SharedStats,
    reload_tx: watch::Sender<()>,
) -> std::io::Result<()> {
    let sessions: SessionStore = Arc::new(Mutex::new(HashMap::new()));
    let state = Data::new(AppState {
        yaml_path,
        shared_file,
        stats,
        reload_tx,
        web_password: config.password.clone(),
        sessions,
    });
    let cert_key = if let (Some(cert), Some(key)) = (config.cert_path, config.key_path) {
        Some((cert, key))
    } else {
        None
    };
    let bind_addr = format!("0.0.0.0:{}", config.port);
    log::info!("[Web] Starting on http{}://{}", if cert_key.is_some() { "s" } else { "" }, bind_addr);
    let server = HttpServer::new(move || {
        App::new()
            .app_data(state.clone())
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .route("/", web::get().to(get_index))
            .route("/logo.png", web::get().to(get_logo))
            .route("/api/login", web::post().to(post_login))
            .route("/api/logout", web::post().to(post_logout))
            .route("/api/status", web::get().to(get_status))
            .route("/api/config", web::get().to(get_config))
            .route("/api/config", web::put().to(update_config))
            .route("/api/torrents", web::get().to(get_torrents))
            .route("/api/torrents", web::post().to(add_torrent))
            .route("/api/torrents/{idx}", web::put().to(update_torrent))
            .route("/api/torrents/{idx}", web::delete().to(delete_torrent))
            .route("/api/browse", web::get().to(browse))
    });
    if let Some((cert_path, key_path)) = cert_key {
        let cert_data = std::fs::read(&cert_path)?;
        let key_data = std::fs::read(&key_path)?;
        let mut cert_reader = std::io::BufReader::new(cert_data.as_slice());
        let mut key_reader = std::io::BufReader::new(key_data.as_slice());
        let certs: Vec<rustls::pki_types::CertificateDer<'static>> =
            rustls_pemfile::certs(&mut cert_reader)
                .filter_map(|c| c.ok())
                .map(|c| c.into_owned())
                .collect();
        let key = rustls_pemfile::private_key(&mut key_reader)
            .ok()
            .flatten()
            .ok_or_else(|| std::io::Error::other("no private key found"))?
            .clone_key();
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(std::io::Error::other)?;
        server.bind_rustls_0_23(&bind_addr, tls_config)?.run().await
    } else {
        server.bind(&bind_addr)?.run().await
    }
}