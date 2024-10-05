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

pub async fn api_service_keys_get(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Return the keys memory object
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": data.torrent_tracker.get_keys(),
    }))
}

pub async fn api_service_keys_post(request: HttpRequest, path: web::Path<(String,i64,)>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate Key if it's in the path
    let path_key = path.into_inner();
    if path_key.0.len() == 40 && path_key.1 <= 0 {
        let key = match hex2bin(path_key.0.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                "status": format!("invalid key {} or timeout {}", path_key.0, path_key.1)
            })); }
        };

        return match data.torrent_tracker.add_key(key, path_key.1) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "updated key"})) }
        }
    }

    // Check if a body was sent without the key in the path, add the list in the body otherwise
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

    let keys = match serde_json::from_slice::<HashMap<String, i64>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for (key_item, timeout) in keys {
        if key_item.len() == 40 && timeout <= 0 {
            let key = match hex2bin(key_item.clone()) {
                Ok(key) => { InfoHash(key) }
                Err(_) => {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({
                        "status": format!("invalid key {} or timeout {}", key_item, timeout)
                    }))
                }
            };

            match data.torrent_tracker.add_key(key, timeout) {
                true => { keys_output.insert(key, json!({"status": "ok"})); }
                false => { keys_output.insert(key, json!({"status": "updated key"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}

pub async fn api_service_keys_delete(request: HttpRequest, path: web::Path<(String,)>, mut payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate Key if it's in the path
    let path_key = path.into_inner();
    if path_key.0.len() == 40 {
        let key = match hex2bin(path_key.0) {
            Ok(key) => { InfoHash(key) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key"})); }
        };

        return match data.torrent_tracker.remove_key(key) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "already removed key"})) }
        }
    }

    // Check if a body was sent without the key in the path, add the list in the body otherwise
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