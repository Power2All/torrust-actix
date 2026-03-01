use crate::config::structs::global_config::GlobalConfig;
use crate::config::structs::torrent_entry::TorrentEntry;
use crate::config::structs::torrents_file::TorrentsFile;
use crate::stats::shared_stats::SharedStats;
use crate::web::structs::app_state::AppState;
use actix_web::{
    web::{
        Data,
        Json,
        Path,
        Payload,
        Query,
    },
    HttpRequest,
    HttpResponse,
};
use argon2::{Argon2, PasswordHash, PasswordVerifier};
use futures_util::StreamExt as _;
use serde::Deserialize;
use serde_json::json;
use std::collections::VecDeque;
use std::io;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::broadcast;

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

// ── WebSocket ─────────────────────────────────────────────────────────────────

#[derive(Deserialize)]
pub struct WsQuery {
    pub token: Option<String>,
}

pub async fn get_ws(
    req: HttpRequest,
    stream: Payload,
    data: Data<AppState>,
    query: Query<WsQuery>,
) -> Result<HttpResponse, actix_web::Error> {
    // Auth: check query-param token (headers can't be set by WebSocket API)
    let authenticated = if data.web_password.is_none() {
        true
    } else {
        let token = query.token.as_deref().unwrap_or("");
        if token.is_empty() {
            false
        } else {
            let sessions = data.sessions.lock().await;
            sessions.get(token).map(|exp| Instant::now() < *exp).unwrap_or(false)
        }
    };
    if !authenticated {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let (res, session, msg_stream) = actix_ws::handle(&req, stream)?;

    // Collect buffered log lines *before* subscribing so we don't miss any
    // and don't hold the std Mutex across an await.
    let buffered: Vec<String> = {
        let buf = data.log_buffer.lock().unwrap_or_else(|e| e.into_inner());
        buf.iter().cloned().collect()
    };
    let log_rx = data.log_tx.subscribe();
    let stats = Arc::clone(&data.stats);

    actix_web::rt::spawn(async move {
        ws_loop(session, msg_stream, log_rx, stats, buffered).await;
    });

    Ok(res)
}

async fn ws_loop(
    mut session: actix_ws::Session,
    mut stream: actix_ws::MessageStream,
    mut log_rx: broadcast::Receiver<String>,
    stats: SharedStats,
    buffered_logs: Vec<String>,
) {
    // Send buffered log history first so the client has context.
    for line in &buffered_logs {
        let msg = serde_json::to_string(&json!({ "type": "log", "line": line }))
            .unwrap_or_default();
        if session.text(msg).await.is_err() {
            return;
        }
    }

    let mut interval = tokio::time::interval(Duration::from_secs(1));
    let mut last_uploaded: u64 = 0;
    let mut last_tick = Instant::now();

    loop {
        tokio::select! {
            _ = interval.tick() => {
                let ts = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_secs();
                let st = stats.read().await;
                let total_peers: usize = st.values().map(|v| v.peer_count).sum();
                let total_uploaded: u64 = st.values().map(|v| v.uploaded).sum();
                drop(st);

                let elapsed = last_tick.elapsed().as_secs_f64().max(0.001);
                let rate = if total_uploaded >= last_uploaded {
                    ((total_uploaded - last_uploaded) as f64 / elapsed) as u64
                } else {
                    0
                };
                last_uploaded = total_uploaded;
                last_tick = Instant::now();

                let msg = serde_json::to_string(&json!({
                    "type": "stats",
                    "ts":    ts,
                    "peers": total_peers,
                    "rate":  rate,
                })).unwrap_or_default();
                if session.text(msg).await.is_err() { break; }
            }

            result = log_rx.recv() => {
                match result {
                    Ok(line) => {
                        let msg = serde_json::to_string(&json!({ "type": "log", "line": line }))
                            .unwrap_or_default();
                        if session.text(msg).await.is_err() { break; }
                    }
                    Err(broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }

            msg = stream.next() => {
                match msg {
                    Some(Ok(actix_ws::Message::Close(reason))) => {
                        let _ = session.close(reason).await;
                        return;
                    }
                    Some(Ok(actix_ws::Message::Ping(bytes))) => {
                        if session.pong(&bytes).await.is_err() { break; }
                    }
                    None | Some(Err(_)) => break,
                    _ => {}
                }
            }
        }
    }

    let _ = session.close(None).await;
}

// Silence unused-import warnings; VecDeque is referenced via the AppState field type.
const _: fn() = || {
    let _: VecDeque<String>;
};

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