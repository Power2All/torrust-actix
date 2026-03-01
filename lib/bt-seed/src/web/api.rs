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
    HttpResponse,
};
use serde::Deserialize;
use serde_json::json;
use std::io;

#[derive(Deserialize)]
pub struct BrowseQuery {
    pub path: Option<String>,
}

pub async fn browse(query: Query<BrowseQuery>) -> HttpResponse {
    let raw = query.path.as_deref().unwrap_or("/");
    let dir = std::path::Path::new(raw);
    let read_dir = match std::fs::read_dir(dir) {
        Ok(rd) => rd,
        Err(e) => return HttpResponse::BadRequest().body(e.to_string()),
    };
    let mut dir_entries: Vec<_> = read_dir.filter_map(|e| e.ok()).collect();
    // Directories first, then files; both alphabetically case-insensitive
    dir_entries.sort_by_key(|e| {
        let is_file = e.file_type().map(|t| t.is_file()).unwrap_or(false);
        (is_file as u8, e.file_name().to_string_lossy().to_lowercase())
    });
    let mut entries: Vec<serde_json::Value> = Vec::new();
    for entry in dir_entries {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name.starts_with('.') { continue; } // skip hidden
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

pub async fn get_status(data: Data<AppState>) -> HttpResponse {
    let stats = data.stats.read().await;
    let map: std::collections::HashMap<String, serde_json::Value> = stats
        .iter()
        .map(|(k, v)| {
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

pub async fn get_torrents(data: Data<AppState>) -> HttpResponse {
    let file = data.shared_file.read().await;
    HttpResponse::Ok().json(&file.torrents)
}

pub async fn add_torrent(data: Data<AppState>, body: Json<TorrentEntry>) -> HttpResponse {
    let mut file = data.shared_file.write().await;
    file.torrents.push(body.into_inner());
    if let Err(e) = write_yaml(&data.yaml_path, &file) {
        return HttpResponse::InternalServerError().body(e.to_string());
    }
    let _ = data.reload_tx.send(());
    HttpResponse::Ok().json(json!({"ok": true}))
}

pub async fn update_torrent(
    data: Data<AppState>,
    idx: Path<usize>,
    body: Json<TorrentEntry>,
) -> HttpResponse {
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

pub async fn delete_torrent(data: Data<AppState>, idx: Path<usize>) -> HttpResponse {
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