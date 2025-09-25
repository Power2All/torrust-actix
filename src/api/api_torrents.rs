use crate::tracker::structs::peer_id::PeerId;
use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use serde_json::{json, Value};
use crate::api::api::{api_parse_body, api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::hex2bin;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

#[tracing::instrument(level = "debug")]
pub async fn api_service_torrent_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let info = path.into_inner();
    if info.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}));
    }

    let info_hash = match hex2bin(info) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
    };

    match data.torrent_tracker.get_torrent(info_hash) {
        None => HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"})),
        Some(torrent) => HttpResponse::Ok().content_type(ContentType::json()).json(api_service_torrents_return_torrent_json(torrent)),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_torrents_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };

    let info_hashes = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(hash) => hash,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };

    let mut torrents_output = HashMap::with_capacity(info_hashes.len());
    for info in info_hashes {
        if info.len() == 40 {
            match hex2bin(info.clone()) {
                Ok(hash) => {
                    let info_hash = InfoHash(hash);
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

#[tracing::instrument(level = "debug")]
pub async fn api_service_torrent_post(request: HttpRequest, path: web::Path<(String, u64)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let (info, completed) = path.into_inner();
    if info.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}));
    }

    let info_hash = match hex2bin(info) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
    };

    let torrent_entry = TorrentEntry {
        seeds: BTreeMap::new(),
        peers: BTreeMap::new(),
        completed,
        updated: std::time::Instant::now(),
    };

    if data.torrent_tracker.config.database.persistent {
        let _ = data.torrent_tracker.add_torrent_update(info_hash, torrent_entry.clone(), UpdatesAction::Add);
    }

    match data.torrent_tracker.add_torrent(info_hash, torrent_entry) {
        (_, true) => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
        (_, false) => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "info_hash updated"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_torrents_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };

    let info_hashmap = match serde_json::from_slice::<HashMap<String, u64>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };

    let mut torrents_output = HashMap::with_capacity(info_hashmap.len());
    for (info, completed) in info_hashmap {
        if info.len() == 40 {
            match hex2bin(info.clone()) {
                Ok(hash) => {
                    let info_hash = InfoHash(hash);
                    let torrent_entry = TorrentEntry {
                        seeds: BTreeMap::new(),
                        peers: BTreeMap::new(),
                        completed,
                        updated: std::time::Instant::now(),
                    };

                    if data.torrent_tracker.config.database.persistent {
                        let _ = data.torrent_tracker.add_torrent_update(info_hash, torrent_entry.clone(), UpdatesAction::Add);
                    }

                    let status = match data.torrent_tracker.add_torrent(info_hash, torrent_entry) {
                        (_, true) => json!({"status": "ok"}),
                        (_, false) => json!({"status": "info_hash updated"}),
                    };
                    torrents_output.insert(info, status);
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

#[tracing::instrument(level = "debug")]
pub async fn api_service_torrent_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let info = path.into_inner();
    if info.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}));
    }

    let info_hash = match hex2bin(info) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
    };

    if data.torrent_tracker.config.database.persistent {
        let _ = data.torrent_tracker.add_torrent_update(info_hash, TorrentEntry::default(), UpdatesAction::Remove);
    }

    match data.torrent_tracker.remove_torrent(info_hash) {
        None => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"})),
        Some(_) => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_torrents_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };

    let hashes = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };

    let mut torrents_output = HashMap::with_capacity(hashes.len());
    for info in hashes {
        if info.len() == 40 {
            match hex2bin(info.clone()) {
                Ok(hash) => {
                    let info_hash = InfoHash(hash);

                    if data.torrent_tracker.config.database.persistent {
                        let _ = data.torrent_tracker.add_torrent_update(info_hash, TorrentEntry::default(), UpdatesAction::Remove);
                    }

                    let status = match data.torrent_tracker.remove_torrent(info_hash) {
                        None => json!({"status": "unknown info_hash"}),
                        Some(_) => json!({"status": "ok"}),
                    };
                    torrents_output.insert(info, status);
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

#[tracing::instrument(level = "debug")]
pub fn api_service_torrents_return_torrent_json(torrent: TorrentEntry) -> Value
{
    let now = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis();

    let process_seed = |(peer_id, torrent_peer): (&PeerId, &crate::tracker::structs::torrent_peer::TorrentPeer)| {
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

    let process_peer = |(peer_id, torrent_peer): (&PeerId, &crate::tracker::structs::torrent_peer::TorrentPeer)| {
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

    let seeds: Vec<Value> = torrent.seeds.iter().map(process_seed).collect();
    let peers: Vec<Value> = torrent.peers.iter().map(process_peer).collect();

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