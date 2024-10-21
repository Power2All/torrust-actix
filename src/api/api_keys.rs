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
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;

pub async fn api_service_key_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let key = path.into_inner();
    if key.len() == 40 {
        let key_hash = match hex2bin(key.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key_hash {}", key)})); }
        };

        return match data.torrent_tracker.get_key(key_hash) {
            None => {
                HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": format!("unknown key_hash {}", key)}))
            }
            Some((_, timeout)) => {
                HttpResponse::Ok().content_type(ContentType::json()).json(json!({
                    "status": "ok",
                    "timeout": timeout
                }))
            }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}))
}

pub async fn api_service_keys_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let keys = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for key in keys {
        if key.len() == 40 {
            let key_hash = match hex2bin(key.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key_hash {}", key)})) }
            };

            keys_output.insert(key_hash, match data.torrent_tracker.get_key(key_hash) {
                None => { 0u64 }
                Some((_, timeout)) => { timeout as u64 }
            });
        }
    }

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

    let (key, timeout) = path.into_inner();
    if key.len() == 40 {
        let key_hash = match hex2bin(key.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key_hash {}", key)})); }
        };

        if data.torrent_tracker.config.database.clone().unwrap().persistent {
            let _ = data.torrent_tracker.add_key_update(key_hash, timeout as i64, UpdatesAction::Add);
        }

        return match data.torrent_tracker.add_key(key_hash, timeout as i64) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("key_hash updated {}", key)})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}))
}

pub async fn api_service_keys_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let keys = match serde_json::from_slice::<HashMap<String, u64>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for (key, timeout) in keys {
        if key.len() == 40 {
            let key_hash = match hex2bin(key.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key_hash {}", key)})) }
            };

            if data.torrent_tracker.config.database.clone().unwrap().persistent {
                let _ = data.torrent_tracker.add_key_update(key_hash, timeout as i64, UpdatesAction::Add);
            }

            match data.torrent_tracker.add_key(key_hash, timeout as i64) {
                true => { keys_output.insert(key, json!({"status": "ok"})); }
                false => { keys_output.insert(key, json!({"status": "key_hash updated"})); }
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

    let key = path.into_inner();
    if key.len() == 40 {
        let key_hash = match hex2bin(key.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key {}", key)})); }
        };

        if data.torrent_tracker.config.database.clone().unwrap().persistent {
            let _ = data.torrent_tracker.add_key_update(key_hash, 0i64, UpdatesAction::Remove);
        }

        return match data.torrent_tracker.remove_key(key_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("unknown key_hash {}", key)})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}))
}

pub async fn api_service_keys_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let keys = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut keys_output = HashMap::new();
    for key_item in keys {
        if key_item.len() == 40 {
            let key_hash = match hex2bin(key_item.clone()) {
                Ok(key) => { InfoHash(key) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key {}", key_item)})) }
            };

            if data.torrent_tracker.config.database.clone().unwrap().persistent {
                let _ = data.torrent_tracker.add_key_update(key_hash, 0i64, UpdatesAction::Remove);
            }

            match data.torrent_tracker.remove_key(key_hash) {
                true => { keys_output.insert(key_hash, json!({"status": "ok"})); }
                false => { keys_output.insert(key_hash, json!({"status": "unknown key_hash"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}