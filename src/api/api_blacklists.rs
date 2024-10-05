use std::collections::HashMap;
use std::sync::Arc;
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

pub async fn api_service_blacklists_get(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Return the blacklist memory object
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "blacklists": data.torrent_tracker.get_blacklist()
    }))
}

pub async fn api_service_blacklists_post(request: HttpRequest, path: web::Path<(String,)>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let path_info_hash = path.into_inner();
    if path_info_hash.0.len() == 40 {
        let info_hash = match hex2bin(path_info_hash.0) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})); }
        };

        return match data.torrent_tracker.add_blacklist(info_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "already blacklisted"})) }
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

            match data.torrent_tracker.add_blacklist(info_hash) {
                true => { torrents_output.insert(info_hash, json!({"status": "ok"})); }
                false => { torrents_output.insert(info_hash, json!({"status": "already blacklisted"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}

pub async fn api_service_blacklists_delete(request: HttpRequest, path: web::Path<(String,)>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let path_info_hash = path.into_inner();
    if path_info_hash.0.len() == 40 {
        let info_hash = match hex2bin(path_info_hash.0) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})); }
        };

        return match data.torrent_tracker.remove_blacklist(info_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "already removed blacklist"})) }
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

            match data.torrent_tracker.remove_blacklist(info_hash) {
                true => { torrents_output.insert(info_hash, json!({"status": "ok"})); }
                false => { torrents_output.insert(info_hash, json!({"status": "already removed blacklist"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": torrents_output
    }))
}