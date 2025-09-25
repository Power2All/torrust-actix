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
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

lazy_static::lazy_static! {
    static ref UUID_REGEX: Regex = Regex::new(r"^[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}$").unwrap();
}

#[tracing::instrument(level = "debug")]
pub async fn api_service_user_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let id = path.into_inner();
    let (status_code, data) = api_service_users_return_json(id, data);
    match status_code {
        StatusCode::OK => HttpResponse::Ok().content_type(ContentType::json()).json(data),
        _ => HttpResponse::NotFound().content_type(ContentType::json()).json(data),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_users_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };

    let ids = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(id) => id,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };

    let mut users_output = HashMap::with_capacity(ids.len());
    for id in ids {
        if id.len() == 40 {
            let (_, user_data) = api_service_users_return_json(id.clone(), Data::clone(&data));
            users_output.insert(id, user_data);
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "users": users_output
    }))
}

#[tracing::instrument(level = "debug")]
pub async fn api_service_user_post(request: HttpRequest, path: web::Path<(String, String, u64, u64, u64, u64, u8)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let (id, key, uploaded, downloaded, completed, updated, active) = path.into_inner();
    if key.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}));
    }

    let key_hash = match hex2bin(key) {
        Ok(hash) => UserId(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key_hash"})),
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

    let id_hash = if data.torrent_tracker.config.database_structure.users.id_uuid {
        if !UUID_REGEX.is_match(&id) {
            return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid uuid"}));
        }
        user_entry.user_uuid = Some(id.to_lowercase());
        hash_id(&id)
    } else {
        match id.parse::<u64>() {
            Ok(user_id) => {
                user_entry.user_id = Some(user_id);
                hash_id(&id)
            }
            Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid id"})),
        }
    };

    if data.torrent_tracker.config.database.persistent {
        let _ = data.torrent_tracker.add_user_update(UserId(id_hash), user_entry.clone(), UpdatesAction::Add);
    }

    match data.torrent_tracker.add_user(UserId(id_hash), user_entry) {
        true => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "user_hash added"})),
        false => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "user_hash updated"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_users_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };

    let hashes = match serde_json::from_slice::<Vec<(String, String, u64, u64, u64, u64, u8)>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };

    let mut users_output = HashMap::with_capacity(hashes.len());
    for (id, key, uploaded, downloaded, completed, updated, active) in hashes {
        if key.len() == 40 {
            let key_hash = match hex2bin(key) {
                Ok(hash) => UserId(hash),
                Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key_hash"})),
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

            let id_hash = if data.torrent_tracker.config.database_structure.users.id_uuid {
                if !UUID_REGEX.is_match(&id) {
                    return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid uuid"}));
                }
                user_entry.user_uuid = Some(id.to_lowercase());
                hash_id(&id)
            } else {
                match id.parse::<u64>() {
                    Ok(user_id) => {
                        user_entry.user_id = Some(user_id);
                        hash_id(&id)
                    }
                    Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid id"})),
                }
            };

            if data.torrent_tracker.config.database.persistent {
                let _ = data.torrent_tracker.add_user_update(UserId(id_hash), user_entry.clone(), UpdatesAction::Add);
            }

            let status = match data.torrent_tracker.add_user(UserId(id_hash), user_entry) {
                true => json!({"status": "user_hash added"}),
                false => json!({"status": "user_hash updated"}),
            };
            users_output.insert(id, status);
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "users": users_output
    }))
}

#[tracing::instrument(level = "debug")]
pub async fn api_service_user_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let id = path.into_inner();
    if id.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad user_hash"}));
    }

    let id_hash = match hex2bin(id) {
        Ok(hash) => UserId(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid user_hash"})),
    };

    if data.torrent_tracker.config.database.persistent {
        let empty_user = UserEntryItem {
            key: UserId([0u8; 20]),
            user_id: None,
            user_uuid: None,
            uploaded: 0,
            downloaded: 0,
            completed: 0,
            updated: 0,
            active: 0,
            torrents_active: BTreeMap::new(),
        };
        let _ = data.torrent_tracker.add_user_update(id_hash, empty_user, UpdatesAction::Remove);
    }

    match data.torrent_tracker.remove_user(id_hash) {
        None => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown user_hash"})),
        Some(_) => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_users_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }

    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };

    let ids = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };

    let mut users_output = HashMap::with_capacity(ids.len());
    for id in ids {
        if id.len() == 40 {
            let id_hash = match hex2bin(id.clone()) {
                Ok(hash) => UserId(hash),
                Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid user_hash"})),
            };

            if data.torrent_tracker.config.database.persistent {
                let empty_user = UserEntryItem {
                    key: UserId([0u8; 20]),
                    user_id: None,
                    user_uuid: None,
                    uploaded: 0,
                    downloaded: 0,
                    completed: 0,
                    updated: 0,
                    active: 0,
                    torrents_active: BTreeMap::new(),
                };
                let _ = data.torrent_tracker.add_user_update(id_hash, empty_user, UpdatesAction::Remove);
            }

            let status = match data.torrent_tracker.remove_user(id_hash) {
                None => json!({"status": "unknown user_hash"}),
                Some(_) => json!({"status": "ok"}),
            };
            users_output.insert(id, status);
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "users": users_output
    }))
}

#[tracing::instrument(level = "debug")]
pub fn api_service_users_return_json(id: String, data: Data<Arc<ApiServiceData>>) -> (StatusCode, Value)
{
    let id_hash = hash_id(&id);
    let uses_uuid = data.torrent_tracker.config.database_structure.users.id_uuid;

    match data.torrent_tracker.get_user(UserId(id_hash)) {
        None => (StatusCode::NOT_FOUND, json!({"status": "unknown user_hash"})),
        Some(user_data) => {
            let response = if uses_uuid {
                json!({
                    "status": "ok",
                    "uuid": user_data.user_uuid,
                    "key": user_data.key,
                    "uploaded": user_data.uploaded,
                    "downloaded": user_data.downloaded,
                    "completed": user_data.completed,
                    "updated": user_data.updated,
                    "active": user_data.active,
                    "torrents_active": user_data.torrents_active
                })
            } else {
                json!({
                    "status": "ok",
                    "id": user_data.user_id,
                    "key": user_data.key,
                    "uploaded": user_data.uploaded,
                    "downloaded": user_data.downloaded,
                    "completed": user_data.completed,
                    "updated": user_data.updated,
                    "active": user_data.active,
                    "torrents_active": user_data.torrents_active
                })
            };
            (StatusCode::OK, response)
        }
    }
}

fn hash_id(id: &str) -> [u8; 20] {
    let mut hasher = Sha1::new();
    hasher.update(id.as_bytes());
    <[u8; 20]>::try_from(hasher.finalize().as_slice()).unwrap()
}