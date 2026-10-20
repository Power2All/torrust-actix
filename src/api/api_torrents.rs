use crate::api::api::{
    api_guard,
    api_json_body,
    parse_hex_id
};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::peer_id::PeerId;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::torrent_update_data::TorrentUpdateData;
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{
    web,
    HttpRequest,
    HttpResponse
};
use serde_json::{
    json,
    Value
};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::{
    SystemTime,
    UNIX_EPOCH
};

/// `GET /api/torrent/{info_hash}` — returns a torrent's swarm data as JSON.
pub async fn api_service_torrent_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    match data.torrent_tracker.get_torrent(info_hash) {
        None => HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"})),
        Some(torrent) => HttpResponse::Ok().content_type(ContentType::json()).json(api_service_torrents_return_torrent_json(torrent)),
    }
}

/// `GET /api/torrents` — returns swarm data for a JSON array of info-hashes in the body.
pub async fn api_service_torrents_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info_hashes: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let mut torrents_output = HashMap::with_capacity(info_hashes.len());
    for info in info_hashes {
        if info.len() == 40 {
            match info.parse::<InfoHash>() {
                Ok(info_hash) => {
                    if let Some(torrent) = data.torrent_tracker.get_torrent(info_hash) {
                        torrents_output.insert(info, api_service_torrents_return_torrent_json(torrent));
                    }
                }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"}))
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

/// `POST /api/torrent/{info_hash}/{completed}` — inserts a torrent with the given completed count.
pub async fn api_service_torrent_post(request: HttpRequest, path: web::Path<(String, u64)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let (info, completed) = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    // Register first, then queue the update built from the *stored* counts: for a torrent that
    // already exists those are its live peer counts, and queueing a peer-less entry would flush
    // zeroed seed/peer numbers to the database and the cache.
    let (torrent_update, inserted) = data.torrent_tracker.set_torrent_completed(info_hash, completed);
    if data.torrent_tracker.config.database_structure.torrents.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_torrent_update(info_hash, torrent_update, UpdatesAction::Add);
    }
    if inserted {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "info_hash updated"}))
    }
}

/// `POST /api/torrents` — inserts multiple torrents from a JSON `{info_hash: completed}` map.
pub async fn api_service_torrents_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info_hashmap: HashMap<String, u64> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    // Everything is parsed before anything is applied, so a bad entry is answered with 400 and no
    // changes, rather than after a random part of the batch (map order) had already been applied.
    let mut validated = Vec::with_capacity(info_hashmap.len());
    for (info, completed) in info_hashmap {
        if info.len() == 40 {
            match info.parse::<InfoHash>() {
                Ok(info_hash) => validated.push((info, info_hash, completed)),
                Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
            }
        }
    }
    let mut torrents_output = HashMap::with_capacity(validated.len());
    for (info, info_hash, completed) in validated {
        let (torrent_update, inserted) = data.torrent_tracker.set_torrent_completed(info_hash, completed);
        if data.torrent_tracker.config.database_structure.torrents.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
            let _ = data.torrent_tracker.add_torrent_update(info_hash, torrent_update, UpdatesAction::Add);
        }
        let status = if inserted {
            json!({"status": "ok"})
        } else {
            json!({"status": "info_hash updated"})
        };
        torrents_output.insert(info, status);
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

/// `DELETE /api/torrent/{info_hash}` — removes a torrent from the tracker.
pub async fn api_service_torrent_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.config.database_structure.torrents.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_torrent_update(info_hash, TorrentUpdateData::default(), UpdatesAction::Remove);
    }
    match data.torrent_tracker.remove_torrent(info_hash) {
        None => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"})),
        Some(_) => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
    }
}

/// `DELETE /api/torrents` — removes a JSON array of info-hashes from the tracker.
pub async fn api_service_torrents_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let hashes: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    // Parsed in full before anything is removed, as in `POST /api/torrents`.
    let mut validated = Vec::with_capacity(hashes.len());
    for info in hashes {
        if info.len() == 40 {
            match info.parse::<InfoHash>() {
                Ok(info_hash) => validated.push((info, info_hash)),
                Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
            }
        }
    }
    let mut torrents_output = HashMap::with_capacity(validated.len());
    for (info, info_hash) in validated {
        if data.torrent_tracker.config.database_structure.torrents.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
            let _ = data.torrent_tracker.add_torrent_update(info_hash, TorrentUpdateData::default(), UpdatesAction::Remove);
        }
        let status = match data.torrent_tracker.remove_torrent(info_hash) {
            None => json!({"status": "unknown info_hash"}),
            Some(_) => json!({"status": "ok"}),
        };
        torrents_output.insert(info, status);
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

/// Serialises a [`TorrentEntry`] (seeds, peers, completed count) into the API JSON shape.
pub fn api_service_torrents_return_torrent_json(torrent: TorrentEntry) -> Value
{
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();
    let process_peer_entry = |(peer_id, torrent_peer): (&PeerId, &crate::tracker::structs::torrent_peer::TorrentPeer)| {
        let elapsed_ms = torrent_peer.updated.elapsed().as_millis();
        let timestamp = now.saturating_sub(elapsed_ms);
        let timestamp_final = ((timestamp as f64 / 2.0).round() * 2.0) as u64;
        json!({
            "peer_id": peer_id.0,
            "peer_addr": torrent_peer.peer_addr,
            "updated": timestamp_final,
            "uploaded": torrent_peer.uploaded.0 as u64,
            "downloaded": torrent_peer.downloaded.0 as u64,
            "left": torrent_peer.left.0 as u64,
        })
    };
    let seeds: Vec<Value> = torrent.seeds.iter()
        .chain(torrent.seeds_ipv6.iter())
        .map(process_peer_entry)
        .collect();
    let peers: Vec<Value> = torrent.peers.iter()
        .chain(torrent.peers_ipv6.iter())
        .map(process_peer_entry)
        .collect();
    let elapsed_ms = torrent.updated.elapsed().as_millis();
    let timestamp = now.saturating_sub(elapsed_ms);
    let timestamp_final = ((timestamp as f64 / 2.0).round() * 2.0) as u64;
    json!({
        "status": "ok",
        "seeds": seeds,
        "peers": peers,
        "completed": torrent.completed,
        "updated": timestamp_final
    })
}