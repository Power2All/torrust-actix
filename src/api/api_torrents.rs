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
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;

pub async fn api_service_torrent_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let info = path.into_inner();
    if info.len() == 40 {
        let info_hash = match hex2bin(info.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})); }
        };

        match data.torrent_tracker.get_torrent(info_hash) {
            None => { return HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": format!("unknown info_hash {}", info)})); }
            Some(torrent) => { return HttpResponse::Ok().content_type(ContentType::json()).json(api_service_torrents_return_torrent_json(torrent)); }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_torrents_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => { data }
        Err(error) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})); }
    };

    let info_hashes = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(hash) => { hash }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut torrents_output = HashMap::new();
    for info in info_hashes {
        if info.len() == 40 {
            let info_hash = match hex2bin(info.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})) }
            };

            match data.torrent_tracker.get_torrent(info_hash) {
                None => {}
                Some(torrent) => { torrents_output.insert(info_hash, api_service_torrents_return_torrent_json(torrent)); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

pub async fn api_service_torrent_post(request: HttpRequest, path: web::Path<(String, u64)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let (info, completed) = path.into_inner();
    if info.len() == 40 {
        let info_hash = match hex2bin(info.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})); }
        };

        let torrent_entry = TorrentEntry {
            seeds: BTreeMap::new(),
            peers: BTreeMap::new(),
            completed,
            updated: std::time::Instant::now(),
        };

        if data.torrent_tracker.config.database.clone().unwrap().persistent {
            let _ = data.torrent_tracker.add_torrent_update(info_hash, torrent_entry.clone());
        }

        return match data.torrent_tracker.add_torrent(info_hash, torrent_entry.clone()) {
            (_, true) => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            (_, false) => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("info_hash updated {}", info)})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_torrents_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => { data }
        Err(error) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})); }
    };

    let info_hashmap = match serde_json::from_slice::<HashMap<String, u64>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut torrents_output = HashMap::new();
    for (info, completed) in info_hashmap {
        if info.len() == 40 {
            let info_hash = match hex2bin(info.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info) })) }
            };

            let torrent_entry = TorrentEntry {
                seeds: BTreeMap::new(),
                peers: BTreeMap::new(),
                completed,
                updated: std::time::Instant::now(),
            };

            if data.torrent_tracker.config.database.clone().unwrap().persistent {
                let _ = data.torrent_tracker.add_torrent_update(info_hash, torrent_entry.clone());
            }

            match data.torrent_tracker.add_torrent(info_hash, torrent_entry.clone()) {
                (_, true) => { torrents_output.insert(info_hash, json!({"status": "ok"})); }
                (_, false) => { torrents_output.insert(info_hash, json!({"status": "info_hash updated"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

pub async fn api_service_torrent_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let info = path.into_inner();
    if info.len() == 40 {
        let info_hash = match hex2bin(info.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})); }
        };

        return match data.torrent_tracker.remove_torrent(info_hash) {
            None => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("unknown info_hash {}", info)})) }
            Some(_) => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_torrents_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => { data }
        Err(error) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})); }
    };

    let hashes = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut torrents_output = HashMap::new();
    for info in hashes {
        if info.len() == 40 {
            let info_hash = match hex2bin(info.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})) }
            };

            match data.torrent_tracker.remove_torrent(info_hash) {
                None => { torrents_output.insert(info_hash, json!({"status": format!("unknown info_hash {}", info)})); }
                Some(_) => { torrents_output.insert(info_hash, json!({"status": "ok"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

pub fn api_service_torrents_return_torrent_json(torrent: TorrentEntry) -> Value
{
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
    json!({
        "status": "ok",
        "seeds": seeds,
        "peers": peers,
        "completed": torrent.completed,
        "updated": timestamp_final
    })
}