use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use futures_util::StreamExt;
use serde_json::json;
use crate::api::api::{api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::hex2bin;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

pub async fn api_service_torrents_get(request: HttpRequest, path: web::Path<String>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let path_info_hash = path.into_inner();
    if path_info_hash.len() == 40 {
        let info_hash = match hex2bin(path_info_hash) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})); }
        };

        match data.torrent_tracker.get_torrent(info_hash) {
            None => {
                return HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"}));
            }
            Some(torrent) => {
                let seeds = torrent.seeds.iter().map(|(peer_id, torrent_peer)| {
                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - torrent_peer.updated.elapsed().as_millis();
                    let timestamp_calc: f64 = timestamp as f64 / 2_f64;
                    let timestamp_final = (timestamp_calc.round() * 2_f64) as u64;
                    json!({
                        "peer_id": peer_id.clone(),
                        "peer_addr": torrent_peer.peer_addr.clone(),
                        "updated": timestamp_final,
                        "uploaded": torrent_peer.uploaded.0 as u64,
                        "downloaded": torrent_peer.downloaded.0 as u64,
                        "left": torrent_peer.left.0 as u64,
                    })
                }).collect::<Vec<_>>();

                let peers = torrent.peers.iter().map(|(peer_id, torrent_peer)| {
                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - torrent_peer.updated.elapsed().as_millis();
                    let timestamp_calc: f64 = timestamp as f64 / 2_f64;
                    let timestamp_final = (timestamp_calc.round() * 2_f64) as u64;
                    json!({
                        "peer_id": peer_id.clone(),
                        "peer_addr": torrent_peer.peer_addr.clone(),
                        "updated": timestamp_final,
                        "uploaded": torrent_peer.uploaded.0 as u64,
                        "downloaded": torrent_peer.downloaded.0 as u64,
                        "left": torrent_peer.left.0 as u64,
                    })
                }).collect::<Vec<_>>();

                let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - torrent.updated.elapsed().as_millis();
                let timestamp_calc: f64 = timestamp as f64 / 2_f64;
                let timestamp_final = (timestamp_calc.round() * 2_f64) as u64;
                return HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                    "status": "ok",
                    "seeds": seeds,
                    "peers": peers,
                    "completed": torrent.completed,
                    "updated": timestamp_final
                }));
            }
        }
    }

    // Check if a body was sent without the hash in the path, return a list otherwise
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(data) => { data }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "chunk error"})); }
        };
        if (body.len() + chunk.len()) > 1_048_576 {
            return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "body overflow"}));
        }
        body.extend_from_slice(&chunk);
    }

    let torrents = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut torrents_output = HashMap::new();
    for info_hash_torrent in torrents {
        if info_hash_torrent.len() == 40 {
            let info_hash = match hex2bin(info_hash_torrent.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid info_hash {}", info_hash_torrent)
                    }))
                }
            };

            match data.torrent_tracker.get_torrent(info_hash) {
                None => {}
                Some(torrent) => {
                    let seeds = torrent.seeds.iter().map(|(peer_id, torrent_peer)| {
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - torrent_peer.updated.elapsed().as_millis();
                        let timestamp_calc: f64 = timestamp as f64 / 2_f64;
                        let timestamp_final = (timestamp_calc.round() * 2_f64) as u64;
                        json!({
                            "peer_id": peer_id.clone(),
                            "peer_addr": torrent_peer.peer_addr.clone(),
                            "updated": timestamp_final,
                            "uploaded": torrent_peer.uploaded.0 as u64,
                            "downloaded": torrent_peer.downloaded.0 as u64,
                            "left": torrent_peer.left.0 as u64,
                        })
                    }).collect::<Vec<_>>();

                    let peers = torrent.peers.iter().map(|(peer_id, torrent_peer)| {
                        let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - torrent_peer.updated.elapsed().as_millis();
                        let timestamp_calc: f64 = timestamp as f64 / 2_f64;
                        let timestamp_final = (timestamp_calc.round() * 2_f64) as u64;
                        json!({
                            "peer_id": peer_id.clone(),
                            "peer_addr": torrent_peer.peer_addr.clone(),
                            "updated": timestamp_final,
                            "uploaded": torrent_peer.uploaded.0 as u64,
                            "downloaded": torrent_peer.downloaded.0 as u64,
                            "left": torrent_peer.left.0 as u64,
                        })
                    }).collect::<Vec<_>>();

                    let timestamp = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() - torrent.updated.elapsed().as_millis();
                    let timestamp_calc: f64 = timestamp as f64 / 2_f64;
                    let timestamp_final = (timestamp_calc.round() * 2_f64) as u64;
                    torrents_output.insert(info_hash, json!({
                        "seeds": seeds,
                        "peers": peers,
                        "completed": torrent.completed,
                        "updated": timestamp_final
                    }));
                }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

pub async fn api_service_torrents_post(request: HttpRequest, path: web::Path<(String, u64)>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let (path_info_hash, completed) = path.into_inner();
    if path_info_hash.len() == 40 {
        let info_hash = match hex2bin(path_info_hash) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})); }
        };

        let torrent_entry = TorrentEntry {
            seeds: BTreeMap::new(),
            peers: BTreeMap::new(),
            completed,
            updated: std::time::Instant::now(),
        };

        let _ = data.torrent_tracker.add_torrent(info_hash, torrent_entry.clone());
        return match data.torrent_tracker.add_torrent_update(info_hash, torrent_entry) {
            (_, true) => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            (_, false) => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "torrent updated"})) }
        }
    }

    // Check if a body was sent without the hash in the path, add the list in the body otherwise
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(data) => { data }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "chunk error"})); }
        };

        if (body.len() + chunk.len()) > 1_048_576 {
            return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "body overflow"}));
        }

        body.extend_from_slice(&chunk);
    }

    let hashes = match serde_json::from_slice::<HashMap<String, u64>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut torrents_output = HashMap::new();
    for (info_hash_torrent, completed) in hashes {
        if info_hash_torrent.len() == 40 {
            let info_hash = match hex2bin(info_hash_torrent.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid info_hash {}", info_hash_torrent)
                    }))
                }
            };

            let torrent_entry = TorrentEntry {
                seeds: BTreeMap::new(),
                peers: BTreeMap::new(),
                completed,
                updated: std::time::Instant::now(),
            };

            let _ = data.torrent_tracker.add_torrent(info_hash, torrent_entry.clone());
            match data.torrent_tracker.add_torrent_update(info_hash, torrent_entry) {
                (_, true) => { torrents_output.insert(info_hash, json!({"status": "ok"})); }
                (_, false) => { torrents_output.insert(info_hash, json!({"status": "torrent already added"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

pub async fn api_service_torrents_delete(request: HttpRequest, path: web::Path<String>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let path_info_hash = path.into_inner();
    if path_info_hash.len() == 40 {
        let info_hash = match hex2bin(path_info_hash) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})); }
        };

        match data.torrent_tracker.remove_torrent(info_hash) {
            None => {
                return HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "torrent already removed"}))
            }
            Some(torrent_entry) => {
                return match data.torrent_tracker.add_torrent_update(info_hash, torrent_entry) {
                    (_, true) => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
                    (_, false) => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "torrent updated"})) }
                }
            }
        }
    }

    // Check if a body was sent without the hash in the path, add the list in the body otherwise
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = match chunk {
            Ok(data) => { data }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "chunk error"})); }
        };

        if (body.len() + chunk.len()) > 1_048_576 {
            return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "body overflow"}));
        }

        body.extend_from_slice(&chunk);
    }

    let hashes = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut torrents_output = HashMap::new();
    for info_hash_torrent in hashes {
        if info_hash_torrent.len() == 40 {
            let info_hash = match hex2bin(info_hash_torrent.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid info_hash {}", info_hash_torrent)
                    }))
                }
            };

            match data.torrent_tracker.remove_torrent(info_hash) {
                None => {
                    torrents_output.insert(info_hash, json!({"status": "torrent already added"}));
                }
                Some(torrent_entry) => {
                    match data.torrent_tracker.add_torrent_update(info_hash, torrent_entry) {
                        (_, true) => { torrents_output.insert(info_hash, json!({"status": "ok"})); }
                        (_, false) => { torrents_output.insert(info_hash, json!({"status": "torrent already added"})); }
                    }
                }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}