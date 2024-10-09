use std::collections::HashMap;
use std::sync::Arc;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use serde_json::json;
use crate::api::api::{api_parse_body, api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::hex2bin;
use crate::tracker::structs::info_hash::InfoHash;

pub async fn api_service_whitelist_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

        match data.torrent_tracker.check_whitelist(info_hash) {
            true => { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})); }
            false => { return HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "whitelist not found"})); }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_whitelists_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Check if a body was sent without the hash in the path, return a list otherwise
    let body = match api_parse_body(payload).await {
        Ok(data) => { data }
        Err(error) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})); }
    };

    let whitelists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut whitelist_output = HashMap::new();
    for info_hash_torrent in whitelists {
        if info_hash_torrent.len() == 40 {
            let info_hash = match hex2bin(info_hash_torrent.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid info_hash {}", info_hash_torrent)
                    }))
                }
            };

            whitelist_output.insert(info_hash, data.torrent_tracker.check_whitelist(info_hash));
        }
    }

    // Return the whitelist memory object
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "whitelists": whitelist_output
    }))
}

pub async fn api_service_whitelist_post(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

        return match data.torrent_tracker.add_whitelist(info_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "already whitelisted"})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_whitelists_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Check if a body was sent without the hash in the path, add the list in the body otherwise
    let body = match api_parse_body(payload).await {
        Ok(data) => { data }
        Err(error) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})); }
    };

    let whitelists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut whitelists_output = HashMap::new();
    for info_hash_torrent in whitelists {
        if info_hash_torrent.len() == 40 {
            let info_hash = match hex2bin(info_hash_torrent.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid info_hash {}", info_hash_torrent)
                    }))
                }
            };

            match data.torrent_tracker.add_whitelist(info_hash) {
                true => { whitelists_output.insert(info_hash, json!({"status": "ok"})); }
                false => { whitelists_output.insert(info_hash, json!({"status": "already whitelisted"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": whitelists_output
    }))
}

pub async fn api_service_whitelist_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

        return match data.torrent_tracker.remove_whitelist(info_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown whitelist"})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_whitelists_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Check if a body was sent without the hash in the path, add the list in the body otherwise
    let body = match api_parse_body(payload).await {
        Ok(data) => { data }
        Err(error) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})); }
    };

    let whitelists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut whitelists_output = HashMap::new();
    for info_hash_torrent in whitelists {
        if info_hash_torrent.len() == 40 {
            let info_hash = match hex2bin(info_hash_torrent.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid info_hash {}", info_hash_torrent)
                    }))
                }
            };

            match data.torrent_tracker.remove_whitelist(info_hash) {
                true => { whitelists_output.insert(info_hash, json!({"status": "ok"})); }
                false => { whitelists_output.insert(info_hash, json!({"status": "unknown whitelist"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": whitelists_output
    }))
}