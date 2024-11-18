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

#[tracing::instrument(level = "debug")]
pub async fn api_service_blacklist_get(request: HttpRequest, path: web::Path<String>, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let info = path.into_inner();
    if info.len() == 40 {
        let info_hash = match hex2bin(info.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})); }
        };

        match data.torrent_tracker.check_blacklist(info_hash) {
            true => { return HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})); }
            false => { return HttpResponse::NotFound().content_type(ContentType::json()).json(json!({"status": format!("unknown info_hash {}", info)})); }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_blacklists_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let blacklists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut blacklist_output = HashMap::new();
    for blacklist in blacklists {
        if blacklist.len() == 40 {
            let blacklist_hash = match hex2bin(blacklist.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", blacklist)})) }
            };

            blacklist_output.insert(blacklist_hash, data.torrent_tracker.check_blacklist(blacklist_hash));
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
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let info = path.into_inner();
    if info.len() == 40 {
        let info_hash = match hex2bin(info.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})); }
        };

        if data.torrent_tracker.config.database.clone().persistent {
            let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
        }

        return match data.torrent_tracker.add_blacklist(info_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("info_hash updated {}", info)})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

#[tracing::instrument(skip(payload, level = "debug"))]
pub async fn api_service_blacklists_post(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let blacklists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut blacklists_output = HashMap::new();
    for info in blacklists {
        if info.len() == 40 {
            let info_hash = match hex2bin(info.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})) }
            };

            if data.torrent_tracker.config.database.clone().persistent {
                let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Add);
            }

            match data.torrent_tracker.add_blacklist(info_hash) {
                true => { blacklists_output.insert(info_hash, json!({"status": "ok"})); }
                false => { blacklists_output.insert(info_hash, json!({"status": "info_hash updated"})); }
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
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    let info = path.into_inner();
    if info.len() == 40 {
        let info_hash = match hex2bin(info.clone()) {
            Ok(hash) => { InfoHash(hash) }
            Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})); }
        };

        if data.torrent_tracker.config.database.clone().persistent {
            let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Remove);
        }

        return match data.torrent_tracker.remove_blacklist(info_hash) {
            true => { HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "ok"})) }
            false => { HttpResponse::NotModified().content_type(ContentType::json()).json(json!({"status": format!("unknown info_hash {}", info)})) }
        }
    }

    HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad info_hash"}))
}

#[tracing::instrument(skip(payload), level = "debug")]
pub async fn api_service_blacklists_delete(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
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

    let blacklists = match serde_json::from_slice::<Vec<String>>(&body) {
        Ok(data) => { data }
        Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "bad json body"})); }
    };

    let mut blacklists_output = HashMap::new();
    for info in blacklists {
        if info.len() == 40 {
            let info_hash = match hex2bin(info.clone()) {
                Ok(hash) => { InfoHash(hash) }
                Err(_) => { return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": format!("invalid info_hash {}", info)})) }
            };

            if data.torrent_tracker.config.database.clone().persistent {
                let _ = data.torrent_tracker.add_blacklist_update(info_hash, UpdatesAction::Remove);
            }

            match data.torrent_tracker.remove_blacklist(info_hash) {
                true => { blacklists_output.insert(info_hash, json!({"status": "ok"})); }
                false => { blacklists_output.insert(info_hash, json!({"status": "unknown info_hash"})); }
            }
        }
    }

    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "blacklists": blacklists_output
    }))
}