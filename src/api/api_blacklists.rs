use crate::api::api::{api_parse_body, api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::common::common::hex2bin;
use crate::tracker::enums::updates_action::UpdatesAction;
use crate::tracker::structs::info_hash::InfoHash;
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{web, HttpRequest, HttpResponse};
use serde_json::json;
use std::collections::HashMap;
use std::sync::Arc;

#[tracing::instrument(level = "debug")]
pub async fn api_service_blacklist_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let info = path.into_inner();
    if info.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}));
    }
    let info_hash = match hex2bin(info) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
    };
    match data.torrent_tracker.check_blacklist(info_hash) {
        true => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
        false => HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_blacklists_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };
    let blacklists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };
    let mut blacklist_output = HashMap::with_capacity(blacklists.len());
    for blacklist in blacklists {
        if blacklist.len() == 40 {
            match hex2bin(blacklist.clone()) {
                Ok(hash) => {
                    let info_hash = InfoHash(hash);
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

#[tracing::instrument(level = "debug")]
pub async fn api_service_blacklist_post(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let info = path.into_inner();
    if info.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}));
    }
    let info_hash = match hex2bin(info) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
    };
    if data.torrent_tracker.config.database.persistent {
        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
    }
    match data.torrent_tracker.add_blacklist(info_hash) {
        true => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
        false => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "info_hash updated"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_blacklists_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };
    let blacklists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };
    let mut blacklists_output = HashMap::with_capacity(blacklists.len());
    for info in blacklists {
        if info.len() == 40 {
            match hex2bin(info.clone()) {
                Ok(hash) => {
                    let info_hash = InfoHash(hash);
                    if data.torrent_tracker.config.database.persistent {
                        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
                    }
                    let status = match data.torrent_tracker.add_blacklist(info_hash) {
                        true => json!({"status": "ok"}),
                        false => json!({"status": "info_hash updated"}),
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

#[tracing::instrument(level = "debug")]
pub async fn api_service_blacklist_delete(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let info = path.into_inner();
    if info.len() != 40 {
        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}));
    }
    let info_hash = match hex2bin(info) {
        Ok(hash) => InfoHash(hash),
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "invalid info_hash"})),
    };
    if data.torrent_tracker.config.database.persistent {
        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Remove);
    }
    match data.torrent_tracker.remove_blacklist(info_hash) {
        true => HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})),
        false => HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": "unknown info_hash"})),
    }
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_blacklists_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let body = match api_parse_body(payload).await {
        Ok(data) => data,
        Err(error) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()})),
    };
    let blacklists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => data,
        Err(_) => return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})),
    };
    let mut blacklists_output = HashMap::with_capacity(blacklists.len());
    for info in blacklists {
        if info.len() == 40 {
            match hex2bin(info.clone()) {
                Ok(hash) => {
                    let info_hash = InfoHash(hash);
                    if data.torrent_tracker.config.database.persistent {
                        let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Remove);
                    }
                    let status = match data.torrent_tracker.remove_blacklist(info_hash) {
                        true => json!({"status": "ok"}),
                        false => json!({"status": "unknown info_hash"}),
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