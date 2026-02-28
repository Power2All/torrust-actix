use crate::config::structs::torrents_file::TorrentsFile;
use crate::config::structs::web_config::WebConfig;
use crate::stats::shared_stats::SharedStats;
use crate::web::api::{
    add_torrent,
    delete_torrent,
    get_index,
    get_status,
    get_torrents,
    update_torrent,
    AppState,
};
use actix_web::{
    middleware::Condition,
    web::{
        self,
        Data
    },
    App,
    HttpServer,
};
use actix_web_httpauth::middleware::HttpAuthentication;
use std::path::PathBuf;
use std::sync::Arc;
use tokio::sync::{
    watch,
    RwLock
};

pub async fn start(
    config: WebConfig,
    yaml_path: PathBuf,
    shared_file: Arc<RwLock<TorrentsFile>>,
    stats: SharedStats,
    reload_tx: watch::Sender<()>,
) -> std::io::Result<()> {
    let password = config.password.clone();
    let has_password = password.is_some();
    let state = Data::new(AppState {
        yaml_path,
        shared_file,
        stats,
        reload_tx,
    });
    let cert_key = if let (Some(cert), Some(key)) = (config.cert_path, config.key_path) {
        Some((cert, key))
    } else {
        None
    };
    let bind_addr = format!("0.0.0.0:{}", config.port);
    log::info!("[Web] Starting on http{}://{}", if cert_key.is_some() { "s" } else { "" }, bind_addr);
    let server = HttpServer::new(move || {
        let pw = password.clone();
        let state = state.clone();
        let auth = HttpAuthentication::basic(move |req, creds| {
            let expected = pw.clone().unwrap_or_default();
            async move {
                if creds.password().map(|p| p == expected.as_str()).unwrap_or(false) {
                    Ok(req)
                } else {
                    Err((
                        actix_web::error::ErrorUnauthorized("Invalid credentials"),
                        req,
                    ))
                }
            }
        });
        App::new()
            .app_data(state)
            .app_data(web::JsonConfig::default().limit(1024 * 1024))
            .wrap(Condition::new(has_password, auth))
            .route("/", web::get().to(get_index))
            .route("/api/status", web::get().to(get_status))
            .route("/api/torrents", web::get().to(get_torrents))
            .route("/api/torrents", web::post().to(add_torrent))
            .route("/api/torrents/{idx}", web::put().to(update_torrent))
            .route("/api/torrents/{idx}", web::delete().to(delete_torrent))
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