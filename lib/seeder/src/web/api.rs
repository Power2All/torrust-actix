use crate::config::structs::global_config::GlobalConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use crate::config::structs::torrents_file::TorrentsFile;
use crate::web::structs::app_state::AppState;
use actix_web::{
    web::{
        Data,
        Json,
        Path,
        Query,
    },
    HttpRequest,
    HttpResponse,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use serde::Deserialize;
use serde_json::json;
use std::io;
use std::time::{Duration, Instant};

const SESSION_TTL: Duration = Duration::from_secs(3600);

pub fn verify_password(input: &str, stored: &str) -> bool {
    if stored.starts_with("$argon2") {
        match PasswordHash::new(stored) {
            Ok(parsed) => Argon2::default()
                .verify_password(input.as_bytes(), &parsed)
                .is_ok(),
            Err(_) => false,
        }
    } else {
        input == stored
    }
}

fn extract_token(req: &HttpRequest) -> Option<String> {
    req.headers()
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.strip_prefix("Bearer "))
        .map(|s| s.to_string())
}

pub async fn is_authenticated(req: &HttpRequest, data: &Data<AppState>) -> bool {
    if data.web_password.is_none() {
        return true;
    }
    let token = match extract_token(req) {
        Some(t) => t,
        None => return false,
    };
    let mut sessions = data.sessions.lock().await;
    if let Some(expiry) = sessions.get(&token) {
        if Instant::now() < *expiry {
            sessions.insert(token, Instant::now() + SESSION_TTL);
            return true;
        }
        sessions.remove(&token);
    }
    false
}

#[derive(Deserialize)]
pub struct LoginRequest {
    pub password: String,
}

pub async fn post_login(data: Data<AppState>, body: Json<LoginRequest>) -> HttpResponse {
    match &data.web_password {
        None => {
            HttpResponse::Ok().json(json!({"token": "noauth"}))
        }
        Some(expected) => {
            if verify_password(&body.password, expected) {
                let token = generate_token();
                let expiry = Instant::now() + SESSION_TTL;
                data.sessions.lock().await.insert(token.clone(), expiry);
                HttpResponse::Ok().json(json!({"token": token}))
            } else {
                HttpResponse::Unauthorized().json(json!({"error": "Invalid password"}))
            }
        }
    }
}

pub async fn post_logout(req: HttpRequest, data: Data<AppState>) -> HttpResponse {
    if let Some(token) = extract_token(&req) {
        data.sessions.lock().await.remove(&token);
    }
    HttpResponse::Ok().json(json!({"ok": true}))
}

fn generate_token() -> String {
    use rand::RngExt;
    let bytes: [u8; 24] = rand::rng().random();
    hex::encode(bytes)
}

#[derive(Deserialize)]
pub struct BrowseQuery {
    pub path: Option<String>,
}

pub async fn browse(req: HttpRequest, query: Query<BrowseQuery>, data: Data<AppState>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let raw = query.path.as_deref().unwrap_or("");
    let dir_buf;
    let dir = if raw.is_empty() {
        dir_buf = std::env::current_dir()
            .unwrap_or_else(|_| std::path::PathBuf::from("/"));
        dir_buf.as_path()
    } else {
        dir_buf = std::path::PathBuf::from(raw);
        dir_buf.as_path()
    };
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };
    let mut dir_entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    dir_entries.sort_by_key(|e| {
        let is_file = e.file_type().map(|t| t.is_file()).unwrap_or(false);
        (is_file as u8, e.file_name().to_string_lossy().to_lowercase())
    });
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for entry in dir_entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') { continue; }
        let meta = match entry.metadata() { Ok(m) => m, Err(_) => continue };
        let is_dir = meta.is_dir();
        let size = if is_dir { 0 } else { meta.len() };
        entries.push(json!({ "name": name, "is_dir": is_dir, "size": size }));
    }
    let parent = dir.parent().map(|p| p.to_string_lossy().into_owned());
    let current = dir.to_string_lossy().into_owned();
    HttpResponse::Ok().json(json!({
        "path": current,
        "parent": parent,
        "entries": entries,
    }))
}

fn write_yaml(path: &std::path::Path, file: &TorrentsFile) -> io::Result<()> {
    let s = serde_yaml::to_string(file).map_err(io::Error::other)?;
    std::fs::write(path, s)
}

pub async fn get_index() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("text/html; charset=utf-8")
        .body(include_str!("index.html"))
}

pub async fn get_logo() -> HttpResponse {
    HttpResponse::Ok()
        .content_type("image/png")
        .insert_header(("Cache-Control", "public, max-age=86400"))
        .body(include_bytes!("logo.png").as_ref())
}

pub async fn get_status(req: HttpRequest, data: Data<AppState>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let stats = data.stats.read().await;
    let map: std::collections::HashMap<String, serde_json::Value> = stats
        .iter()
        .map(|(k, v): (&String, &crate::stats::shared_stats::TorrentStats)| {
            (
                k.clone(),
                json!({
                    "uploaded": v.uploaded,
                    "peer_count": v.peer_count,
                }),
            )
        })
        .collect();
    HttpResponse::Ok().json(json!({ "torrents": map }))
}

pub async fn get_torrents(req: HttpRequest, data: Data<AppState>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let file = data.shared_file.read().await;
    HttpResponse::Ok().json(&file.torrents)
}

pub async fn add_torrent(req: HttpRequest, data: Data<AppState>, body: Json<TorrentEntry>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let mut file = data.shared_file.write().await;
    file.torrents.push(body.into_inner());
    if let Err(e) = write_yaml(&data.yaml_path, &file) {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    let _ = data.reload_tx.send(());
    HttpResponse::Ok().json(json!({"ok": true}))
}

pub async fn update_torrent(
    req: HttpRequest,
    data: Data<AppState>,
    idx: Path<usize>,
    body: Json<TorrentEntry>,
) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let mut file = data.shared_file.write().await;
    let i = idx.into_inner();
    if i >= file.torrents.len() {
        return HttpResponse::NotFound().body("index out of range");
    }
    file.torrents[i] = body.into_inner();
    if let Err(e) = write_yaml(&data.yaml_path, &file) {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    let _ = data.reload_tx.send(());
    HttpResponse::Ok().json(json!({"ok": true}))
}

pub async fn delete_torrent(req: HttpRequest, data: Data<AppState>, idx: Path<usize>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let mut file = data.shared_file.write().await;
    let i = idx.into_inner();
    if i >= file.torrents.len() {
        return HttpResponse::NotFound().body("index out of range");
    }
    file.torrents.remove(i);
    if let Err(e) = write_yaml(&data.yaml_path, &file) {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    let _ = data.reload_tx.send(());
    HttpResponse::Ok().json(json!({"ok": true}))
}

pub async fn get_config(req: HttpRequest, data: Data<AppState>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let file = data.shared_file.read().await;
    HttpResponse::Ok().json(&file.config)
}

pub async fn update_config(req: HttpRequest, data: Data<AppState>, body: Json<GlobalConfig>) -> HttpResponse {
    if !is_authenticated(&req, &data).await {
        return HttpResponse::Unauthorized().json(json!({"error": "Unauthorized"}));
    }
    let new_cfg = body.into_inner();
    if let Some(ref level_str) = new_cfg.log_level {
        let filter = match level_str.to_ascii_lowercase().as_str() {
            "error" => log::LevelFilter::Error,
            "warn"  => log::LevelFilter::Warn,
            "debug" => log::LevelFilter::Debug,
            "trace" => log::LevelFilter::Trace,
            _       => log::LevelFilter::Info,
        };
        log::set_max_level(filter);
        log::info!("[Config] Log level set to {}", level_str);
    }
    let mut file = data.shared_file.write().await;
    file.config = new_cfg;
    if let Err(e) = write_yaml(&data.yaml_path, &file) {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    let _ = data.reload_tx.send(());
    HttpResponse::Ok().json(json!({"ok": true}))
}