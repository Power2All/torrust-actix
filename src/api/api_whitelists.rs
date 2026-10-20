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

/// Validates a batch of info-hash strings up front (before any state is mutated), so a bad
/// entry can never leave earlier entries half-applied.
///
/// # Errors
///
/// Returns the `BadRequest` response to send when an entry is not 40 hex characters.
fn api_whitelists_validate(whitelists: Vec<String>) -> Result<Vec<(String, InfoHash)>, Box<HttpResponse>> {
    let mut validated = Vec::with_capacity(whitelists.len());
    for info in whitelists {
        let info_hash = parse_hex_id(&info, "info_hash")?;
        validated.push((info, info_hash));
    }
    Ok(validated)
}

/// `GET /api/whitelist/{info_hash}` — returns whether the info-hash is whitelisted.
pub async fn api_service_whitelist_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.check_whitelist(info_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"}))
    }
}

/// `GET /api/whitelists` — checks a JSON array of info-hashes against the whitelist.
pub async fn api_service_whitelists_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let whitelists: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let validated = match api_whitelists_validate(whitelists) {
        Ok(validated) => validated,
        Err(response) => return *response,
    };
    let mut whitelist_output = HashMap::with_capacity(validated.len());
    for (whitelist, whitelist_hash) in validated {
        whitelist_output.insert(whitelist, data.torrent_tracker.check_whitelist(whitelist_hash));
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "whitelists": whitelist_output
    }))
}

/// `POST /api/whitelist/{info_hash}` — adds an info-hash to the whitelist.
pub async fn api_service_whitelist_post(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.config.database_structure.whitelist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_whitelist_update(info_hash, UpdatesAction::Add);
    }
    if data.torrent_tracker.add_whitelist(info_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "info_hash updated"}))
    }
}

/// `POST /api/whitelists` — adds a JSON array of info-hashes to the whitelist.
pub async fn api_service_whitelists_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let whitelists: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let validated = match api_whitelists_validate(whitelists) {
        Ok(validated) => validated,
        Err(response) => return *response,
    };
    let mut whitelists_output = HashMap::with_capacity(validated.len());
    for (info, info_hash) in validated {
        if data.torrent_tracker.config.database_structure.whitelist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
            let _ = data.torrent_tracker.add_whitelist_update(info_hash, UpdatesAction::Add);
        }
        let status = if data.torrent_tracker.add_whitelist(info_hash) {
            json!({"status": "ok"})
        } else {
            json!({"status": "info_hash updated"})
        };
        whitelists_output.insert(info, status);
    }
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "whitelists": whitelists_output
    }))
}

/// `DELETE /api/whitelist/{info_hash}` — removes an info-hash from the whitelist.
pub async fn api_service_whitelist_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let info = path.into_inner();
    let info_hash: InfoHash = match parse_hex_id(&info, "info_hash") {
        Ok(id) => id,
        Err(response) => return *response,
    };
    if data.torrent_tracker.config.database_structure.whitelist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let _ = data.torrent_tracker.add_whitelist_update(info_hash, UpdatesAction::Remove);
    }
    if data.torrent_tracker.remove_whitelist(info_hash) {
        HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
    } else {
        HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"}))
    }
}

/// `DELETE /api/whitelists` — removes a JSON array of info-hashes from the whitelist.
pub async fn api_service_whitelists_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    let whitelists: Vec<String> = match api_json_body(payload).await {
        Ok(body) => body,
        Err(response) => return *response,
    };
    let validated = match api_whitelists_validate(whitelists) {
        Ok(validated) => validated,
        Err(response) => return *response,
    };
    let mut whitelists_output = HashMap::with_capacity(validated.len());
    for (info, info_hash) in validated {
        if data.torrent_tracker.config.database_structure.whitelist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
            let _ = data.torrent_tracker.add_whitelist_update(info_hash, UpdatesAction::Remove);
        }
        let status = if data.torrent_tracker.remove_whitelist(info_hash) {
            json!({"status": "ok"})
        } else {
            json!({"status": "unknown info_hash"})
        };
        whitelists_output.insert(info, status);
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "whitelists": whitelists_output
    }))
}
/// `DELETE /api/whitelist/clear` — empties the whitelist.
pub async fn api_service_whitelist_clear(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(response) = api_guard(&request, &data).await { return response; }
    if data.torrent_tracker.config.database_structure.whitelist.persistent.unwrap_or(data.torrent_tracker.config.database.persistent) {
        let table = data.torrent_tracker.config.database_structure.whitelist.table_name.clone();
        if data.torrent_tracker.sqlx.clear_table(&table).await.is_err() {
            return HttpResponse::InternalServerError().content_type(ContentType::json()).json(json!({"status": "database error"}));
        }
    }
    data.torrent_tracker.clear_whitelist();
    data.torrent_tracker.clear_whitelist_updates();
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"}))
}
