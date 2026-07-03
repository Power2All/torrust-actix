use crate::api::api::{
    api_parse_body,
    api_service_token,
    api_validation
};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::common::common::hex2bin;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{
    web,
    HttpRequest,
    HttpResponse
};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

/// `GET /api/key/{key_hash}` — returns whether the announce key exists and its expiry.
pub async fn api_service_key_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let key = path.into_inner();
    if key.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}));
    }
    let key_hash = match hex2bin(key) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key_hash"})),
    };
    match data.torrent_tracker.get_key(key_hash) {
        None => HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown key_hash"})),
        Some((_, timeout)) => HttpResponse::Ok().content_type(ContentType::json()).json(json!({
            "status": "ok",
            "timeout": timeout
        })),
    }
}

/// `GET /api/keys` — checks a JSON array of key hashes against the key table.
pub async fn api_service_keys_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };
    let keys = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };
    let mut keys_output = HashMap::with_capacity(keys.len());
    for key in keys {
        if key.len() == 40 {
            match hex2bin(key.clone()) {
                Ok(hash) => {
                    let key_hash = InfoHash(hash);
                    let timeout = data.torrent_tracker.get_key(key_hash)
                        .map_or(0u64, |(_, timeout)| timeout as u64);
                    keys_output.insert(key, timeout);
                }
                Err(_) => {
                    keys_output.insert(key, 0u64);
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}

/// `POST /api/key/{key_hash}/{timeout}` — adds an announce key valid for `timeout` seconds
/// (0 = permanent).
pub async fn api_service_key_post(request: HttpRequest, path: web::Path<(String, u64)>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let (key, timeout) = path.into_inner();
    if key.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}));
    }
    let key_hash = match hex2bin(key) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key_hash"})),
    };
    if data.torrent_tracker.config.database_structure.keys.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_key_update(key_hash, timeout as i64, UpdatesAction::Add);
    }
    if data.torrent_tracker.add_key(key_hash, timeout as i64) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "key_hash updated"}))
    }
}

/// `POST /api/keys` — adds a JSON `{key_hash: timeout}` map of announce keys.
pub async fn api_service_keys_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };
    let keys = match serde_json::from_slice::<HashMap<String, u64>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };
    let mut keys_output = HashMap::with_capacity(keys.len());
    for (key, timeout) in keys {
        if key.len() == 40 {
            match hex2bin(key.clone()) {
                Ok(hash) => {
                    let key_hash = InfoHash(hash);
                    if data.torrent_tracker.config.database_structure.keys.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
                        let _ = data.torrent_tracker.add_key_update(key_hash, timeout as i64, UpdatesAction::Add);
                    }
                    let status = if data.torrent_tracker.add_key(key_hash, timeout as i64) {
                        json!({"status": "ok"})
                    } else {
                        json!({"status": "key_hash updated"})
                    };
                    keys_output.insert(key, status);
                }
                Err(_) => {
                    keys_output.insert(key, json!({"status": "invalid key_hash"}));
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}

/// `DELETE /api/key/{key_hash}` — removes an announce key.
pub async fn api_service_key_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let key = path.into_inner();
    if key.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad key_hash"}));
    }
    let key_hash = match hex2bin(key) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid key"})),
    };
    if data.torrent_tracker.config.database_structure.keys.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_key_update(key_hash, 0i64, UpdatesAction::Remove);
    }
    if data.torrent_tracker.remove_key(key_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown key_hash"}))
    }
}

/// `DELETE /api/keys` — removes a JSON array of announce keys.
pub async fn api_service_keys_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };
    let keys = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };
    let mut keys_output = HashMap::with_capacity(keys.len());
    for key_item in keys {
        if key_item.len() == 40 {
            match hex2bin(key_item.clone()) {
                Ok(hash) => {
                    let key_hash = InfoHash(hash);
                    if data.torrent_tracker.config.database_structure.keys.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
                        let _ = data.torrent_tracker.add_key_update(key_hash, 0i64, UpdatesAction::Remove);
                    }
                    let status = if data.torrent_tracker.remove_key(key_hash) {
                        json!({"status": "ok"})
                    } else {
                        json!({"status": "unknown key_hash"})
                    };
                    keys_output.insert(key_item, status);
                }
                Err(_) => {
                    keys_output.insert(key_item, json!({"status": "invalid key"}));
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "keys": keys_output
    }))
}
/// `DELETE /api/keys/clear` — empties the key table.
pub async fn api_service_keys_clear(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    if let Some(response) = api_service_token(&request, Arc::clone(&data.torrent_tracker.config)).await { return response; }
    if !data.torrent_tracker.config.tracker_config.keys_enabled {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "keys not enabled"}));
    }
    if data.torrent_tracker.config.database_structure.keys.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let table = data.torrent_tracker.config.database_structure.keys.table_name.clone();
        if data.torrent_tracker.sqlx.clear_table(&table).await.is_err() {
            return HttpResponse::InternalServerError().content_type(ContentType::json()).json(json!({"status": "database error"}));
        }
    }
    data.torrent_tracker.clear_keys();
    data.torrent_tracker.clear_key_updates();
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}
