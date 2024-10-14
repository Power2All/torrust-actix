use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use regex::Regex;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use crate::api::api::{api_parse_body, api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::hex2bin;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

pub async fn api_service_user_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let id = path.into_inner();
    let (status_code, data) = api_service_users_return_json(id, data.clone());
    match status_code {
        StatusCode::OK => { HttpResponse::Ok().content_type(ContentType::json()).json(data) }
        _ => { HttpResponse::NotFound().content_type(ContentType::json()).json(data) }
    }
}

pub async fn api_service_users_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let ids = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(id) => { id }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut users_output = HashMap::new();
    for id in ids {
        if id.len() == 40 {
            let (_, data) = api_service_users_return_json(id.clone(), data.clone());
            users_output.insert(id.clone(), data);
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "users": users_output
    }))
}

pub async fn api_service_user_post(request: HttpRequest, path: web::Path<(String, String, u64, u64, u64, u64, u8)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let (id, key, uploaded, downloaded, completed, updated, active) = path.into_inner();
    if key.len() == 40 {
        let key_hash = match hex2bin(key.clone()) {
            Ok(hash) => { UserId(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key_hash {}", key)})); }
        };

        let mut user_entry = UserEntryItem {
            key: key_hash,
            user_id: None,
            user_uuid: None,
            uploaded,
            downloaded,
            completed,
            updated,
            active,
            torrents_active: BTreeMap::new(),
        };

        match data.torrent_tracker.config.database_structure.clone().unwrap().users.unwrap().id_uuid {
            true => {
                let regex_check = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
                if !regex_check.is_match(id.as_str()) {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid uuid {}", id)}));
                }
                user_entry.user_uuid = Some(id.to_lowercase());
            }
            false => {
                match id.parse::<u64>() {
                    Ok(data) => { user_entry.user_id = Some(data); }
                    Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid id {}", id)})); }
                }
            }
        }

        let id_data: &[u8] = id.as_bytes();
        let mut hasher = Sha1::new();
        hasher.update(id_data);
        let id_hash = <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap();

        if data.torrent_tracker.config.database.clone().unwrap().persistent {
            let _ = data.torrent_tracker.add_user_update(UserId(id_hash), user_entry.clone());
        }

        return match data.torrent_tracker.add_user(UserId(id_hash), user_entry.clone()) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": format!("user_hash added {}", UserId(id_hash))})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("user_hash updated {}", UserId(id_hash))})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}))
}

pub async fn api_service_users_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let hashes = match serde_json::from_slice::<Vec<(String, String, u64, u64, u64, u64, u8)>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut users_output = HashMap::new();
    for (id, key, uploaded, downloaded, completed, updated, active) in hashes {
        if key.len() == 40 {
            let key_hash = match hex2bin(key.clone()) {
                Ok(hash) => { UserId(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid key_hash {}", key)})); }
            };

            let mut user_entry = UserEntryItem {
                key: key_hash,
                user_id: None,
                user_uuid: None,
                uploaded,
                downloaded,
                completed,
                updated,
                active,
                torrents_active: BTreeMap::new(),
            };

            match data.torrent_tracker.config.database_structure.clone().unwrap().users.unwrap().id_uuid {
                true => {
                    let regex_check = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
                    if !regex_check.is_match(id.as_str()) {
                        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid uuid {}", id)}));
                    }
                    user_entry.user_uuid = Some(id.to_lowercase());
                }
                false => {
                    match id.parse::<u64>() {
                        Ok(data) => { user_entry.user_id = Some(data); }
                        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid id {}", id)})); }
                    }
                }
            }

            let id_data: &[u8] = id.as_bytes();
            let mut hasher = Sha1::new();
            hasher.update(id_data);
            let id_hash = <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap();

            if data.torrent_tracker.config.database.clone().unwrap().persistent {
                let _ = data.torrent_tracker.add_user_update(UserId(id_hash), user_entry.clone());
            }

            match data.torrent_tracker.add_user(UserId(id_hash), user_entry.clone()) {
                true => { users_output.insert(UserId(id_hash), json!({"status": "user_hash added"})); }
                false => { users_output.insert(UserId(id_hash), json!({"status": "user_hash updated"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "users": users_output
    }))
}

pub async fn api_service_user_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let id = path.into_inner();
    if id.len() == 40 {
        let id_hash = match hex2bin(id.clone()) {
            Ok(hash) => { UserId(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid user_hash {}", id)})); }
        };

        return match data.torrent_tracker.remove_user(id_hash) {
            None => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("unknown user_hash {}", id)})) }
            Some(_) => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad user_hash"}))
}

pub async fn api_service_users_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let ids = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut users_output = HashMap::new();
    for id in ids {
        if id.len() == 40 {
            let id_hash = match hex2bin(id.clone()) {
                Ok(hash) => { UserId(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid user_hash {}", id)})) }
            };

            match data.torrent_tracker.remove_user(id_hash) {
                None => { users_output.insert(id_hash, json!({"status": "unknown user_hash"})); }
                Some(_) => { users_output.insert(id_hash, json!({"status": "ok"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "users": users_output
    }))
}

pub fn api_service_users_return_json(id: String, data: Data<Arc<ApiServiceData>>) -> (StatusCode, Value)
{
    match data.torrent_tracker.config.database_structure.clone().unwrap().users.unwrap().id_uuid {
        true => {
            let uuid_data: &[u8] = id.as_bytes();
            let mut hasher = Sha1::new();
            hasher.update(uuid_data);
            let uuid_hashed = <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap();

            match data.torrent_tracker.get_user(UserId(uuid_hashed)) {
                None => {
                    (StatusCode::NOT_FOUND, json!({"status": "unknown user_hash"}))
                }
                Some(user_data) => {
                    (StatusCode::OK, json!({
                        "status": "ok",
                        "uuid": user_data.user_uuid,
                        "key": user_data.key,
                        "uploaded": user_data.uploaded,
                        "downloaded": user_data.downloaded,
                        "completed": user_data.completed,
                        "updated": user_data.updated,
                        "active": user_data.active,
                        "torrents_active": user_data.torrents_active
                    }))
                }
            }
        }
        false => {
            let id_data: &[u8] = id.as_bytes();
            let mut hasher = Sha1::new();
            hasher.update(id_data);
            let id_hashed = <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap();

            match data.torrent_tracker.get_user(UserId(id_hashed)) {
                None => {
                    (StatusCode::NOT_FOUND, json!({"status": "unknown user_hash"}))
                }
                Some(user_data) => {
                    (StatusCode::OK, json!({
                        "status": "ok",
                        "id": user_data.user_id,
                        "key": user_data.key,
                        "uploaded": user_data.uploaded,
                        "downloaded": user_data.downloaded,
                        "completed": user_data.completed,
                        "updated": user_data.updated,
                        "active": user_data.active,
                        "torrents_active": user_data.torrents_active
                    }))
                }
            }
        }
    }
}