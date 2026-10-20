use crate::api::api::{
    api_guard,
    api_json_body,
    parse_hex_id
};
use crate::api::structs::api_service_data::ApiServiceData;
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

/// `GET /api/blacklist/{info_hash}` — returns whether the info-hash is blacklisted.
pub async fn api_service_blacklist_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.check_blacklist(info_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"}))
    }
}

/// `GET /api/blacklists` — checks a JSON array of info-hashes against the blacklist.
pub async fn api_service_blacklists_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let blacklists: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let mut blacklist_output = HashMap::with_capacity(blacklists.len());
    for blacklist in blacklists {
        if blacklist.len() == 40 {
            match blacklist.parse::<InfoHash>() {
                Ok(info_hash) => {
                    blacklist_output.insert(blacklist, data.torrent_tracker.check_blacklist(info_hash));
                }
                Err(_) => {
                    blacklist_output.insert(blacklist, false);
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "blacklists": blacklist_output
    }))
}

/// `POST /api/blacklist/{info_hash}` — adds an info-hash to the blacklist.
pub async fn api_service_blacklist_post(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.config.database_structure.blacklist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
    }
    if data.torrent_tracker.add_blacklist(info_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "info_hash updated"}))
    }
}

/// `POST /api/blacklists` — adds a JSON array of info-hashes to the blacklist.
pub async fn api_service_blacklists_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let blacklists: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let mut blacklists_output = HashMap::with_capacity(blacklists.len());
    for info in blacklists {
        if info.len() == 40 {
            match info.parse::<InfoHash>() {
                Ok(info_hash) => {
                    if data.torrent_tracker.config.database_structure.blacklist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
                        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
                    }
                    let status = if data.torrent_tracker.add_blacklist(info_hash) {
                        json!({"status": "ok"}) } else { json!({"status": "info_hash updated"}) };
                    blacklists_output.insert(info, status);
                }
                Err(_) => {
                    blacklists_output.insert(info, json!({"status": "invalid info_hash"}));
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "blacklists": blacklists_output
    }))
}

/// `DELETE /api/blacklist/{info_hash}` — removes an info-hash from the blacklist.
pub async fn api_service_blacklist_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.config.database_structure.blacklist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Remove);
    }
    if data.torrent_tracker.remove_blacklist(info_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"}))
    }
}

/// `DELETE /api/blacklists` — removes a JSON array of info-hashes from the blacklist.
pub async fn api_service_blacklists_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let blacklists: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let mut blacklists_output = HashMap::with_capacity(blacklists.len());
    for info in blacklists {
        if info.len() == 40 {
            match info.parse::<InfoHash>() {
                Ok(info_hash) => {
                    if data.torrent_tracker.config.database_structure.blacklist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
                        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Remove);
                    }
                    let status = if data.torrent_tracker.remove_blacklist(info_hash) {
                        json!({"status": "ok"})
                    } else {
                        json!({"status": "unknown info_hash"})
                    };
                    blacklists_output.insert(info, status);
                }
                Err(_) => {
                    blacklists_output.insert(info, json!({"status": "invalid info_hash"}));
                }
            }
        }
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "blacklists": blacklists_output
    }))
}
/// `DELETE /api/blacklist/clear` — empties the blacklist.
pub async fn api_service_blacklist_clear(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    if data.torrent_tracker.config.database_structure.blacklist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let table = data.torrent_tracker.config.database_structure.blacklist.table_name.clone();
        if data.torrent_tracker.sqlx.clear_table(&table).await.is_err() {
            return HttpResponse::InternalServerError().content_type(ContentType::json()).json(json!({"status": "database error"}));
        }
    }
    data.torrent_tracker.clear_blacklist();
    data.torrent_tracker.clear_blacklist_updates();
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}
