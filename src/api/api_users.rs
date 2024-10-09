use std::collections::{BTreeMap, HashMap};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::http::StatusCode;
use actix_web::web::Data;
use futures_util::StreamExt;
use log::info;
use serde_json::{json, Value};
use sha1::{Digest, Sha1};
use crate::api::api::{api_parse_body, api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::hex2bin;
use crate::common::structs::custom_error::CustomError;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_entry::TorrentEntry;
use crate::tracker::structs::user_entry_item::UserEntryItem;
use crate::tracker::structs::user_id::UserId;

pub async fn api_service_user_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get the ID or UUID requested, based on configuration
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

    let users = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut users_output = HashMap::new();
    for users_hash in users {
        if users_hash.len() == 40 {
            let (_, data) = api_service_users_return_json(users_hash.clone(), data.clone());
            users_output.insert(users_hash.clone(), data);
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "torrents": users_output
    }))
}

pub async fn api_service_user_post(request: HttpRequest, path: web::Path<(String, String, u64, u64, u64, u64, u8)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get and validate InfoHash if it's in the path
    let (id, key, uploaded, downloaded, completed, updated, active) = path.into_inner();
    if id.len() == 40 && key.len() == 40 {
        let id_hash = match hex2bin(id.clone()) {
            Ok(hash) => { UserId(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid id_hash"})); }
        };

        let key_hash = match hex2bin(key) {
            Ok(hash) => { UserId(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key_hash"})); }
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
                user_entry.user_uuid = Some(id.clone());
            }
            false => {
                user_entry.user_id = Some(id.clone().parse().unwrap());
            }
        }

        let _ = data.torrent_tracker.add_user(id_hash, user_entry.clone());
        return match data.torrent_tracker.add_user_update(id_hash, user_entry.clone()) {
            (_, true) => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            (_, false) => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "torrent updated"})) }
        };
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

pub async fn api_service_users_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    HttpResponse::Ok().body("")
}

pub async fn api_service_user_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    HttpResponse::Ok().body("")
}

pub async fn api_service_users_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    HttpResponse::Ok().body("")
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
                    (StatusCode::NOT_FOUND, json!({"status": "unknown user"}))
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
                    (StatusCode::NOT_FOUND, json!({"status": "unknown user"}))
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