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

pub async fn api_service_key_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate Key hash if it's in the path
    let path_key_hash = path.into_inner();
    if path_key_hash.len() == 40 {
        let key_hash = match hex2bin(path_key_hash) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key"})); }
        };

        match data.torrent_tracker.check_key(key_hash) {
            true => { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})); }
            false => { return HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "key not found"})); }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key"}))
}

pub async fn api_service_keys_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let keys = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for key_hash in keys {
        if key_hash.len() == 40 {
            let key = match hex2bin(key_hash.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid key {}", key_hash)
                    }))
                }
            };

            keys_output.insert(key_hash, data.torrent_tracker.check_key(key));
        }
    }

    // Return the whitelist memory object
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}

pub async fn api_service_key_post(request: HttpRequest, path: web::Path<(String, u64)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let (path_key_hash, timeout) = path.into_inner();
    if path_key_hash.len() == 40 {
        let key_hash = match hex2bin(path_key_hash) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key"})); }
        };

        return match data.torrent_tracker.add_key(key_hash, timeout as i64) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "already added key"})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key"}))
}

pub async fn api_service_keys_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let keys = match serde_json::from_slice::<HashMap<String, u64>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for (key_hash, timeout) in keys {
        if key_hash.len() == 40 {
            let key = match hex2bin(key_hash.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid key {}", key_hash)
                    }))
                }
            };

            match data.torrent_tracker.add_key(key, timeout as i64) {
                true => { keys_output.insert(key, json!({"status": "ok"})); }
                false => { keys_output.insert(key, json!({"status": "already added key"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}

pub async fn api_service_key_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate Key if it's in the path
    let path_key = path.into_inner();
    if path_key.len() == 40 {
        let key = match hex2bin(path_key) {
            Ok(key) => { InfoHash(key) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key"})); }
        };

        return match data.torrent_tracker.remove_key(key) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "already removed key"})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key"}))
}

pub async fn api_service_keys_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let keys = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for key_item in keys {
        if key_item.len() == 40 {
            let key = match hex2bin(key_item.clone()) {
                Ok(key) => { InfoHash(key) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid key {}", key_item)
                    }))
                }
            };

            match data.torrent_tracker.remove_key(key) {
                true => { keys_output.insert(key, json!({"status": "ok"})); }
                false => { keys_output.insert(key, json!({"status": "already removed key"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": keys_output
    }))
}