use crate::api::api::{
    api_service_token,
    api_validation
};
use crate::api::structs::api_certificate::{
    CertificateReloadRequest,
    CertificateStatusItem,
    CertificateReloadResult,
    CertificateReloadError
};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use crate::ssl::enums::server_identifier::ServerIdentifier;
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{
    web,
    HttpRequest,
    HttpResponse
};
use serde_json::json;
use std::sync::Arc;

#[tracing::instrument(level = "debug")]
pub async fn api_service_certificate_reload(
    request: HttpRequest,
    data: Data<Arc<ApiServiceData>>,
    body: Option<web::Json<CertificateReloadRequest>>,
) -> HttpResponse {
    if let Some(error_return) = api_validation(&request, &data).await {
        return error_return;
    }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await {
        return response;
    }
    let certificate_store = &data.torrent_tracker.certificate_store;
    let (server_type_filter, bind_address_filter) = match body {
        Some(req) => (req.server_type.clone(), req.bind_address.clone()),
        None => (None, None),
    };
    let certificates_to_reload: Vec<ServerIdentifier> = {
        let certificates = certificate_store.get_all_certificates();
        certificates
            .into_iter()
            .filter(|(server_id, _)| {
                if let Some(ref filter) = server_type_filter
                    && server_id.server_type() != filter.to_lowercase()
                {
                    return false;
                }
                if let Some(ref filter) = bind_address_filter
                    && server_id.bind_address() != filter
                {
                    return false;
                }
                true
            })
            .map(|(server_id, _)| server_id)
            .collect()
    };
    if certificates_to_reload.is_empty() {
        return HttpResponse::Ok().content_type(ContentType::json()).json(json!({
            "status": "no_certificates",
            "message": "No SSL certificates found to reload"
        }));
    }
    let mut reloaded: Vec<CertificateReloadResult> = Vec::with_capacity(certificates_to_reload.len());
    let mut errors: Vec<CertificateReloadError> = Vec::new();
    for server_id in certificates_to_reload {
        match certificate_store.reload_certificate(&server_id) {
            Ok(()) => {
                let loaded_at = certificate_store
                    .get_certificate(&server_id)
                    .map(|bundle| bundle.loaded_at.to_rfc3339())
                    .unwrap_or_else(|| "unknown".to_string());
                reloaded.push(CertificateReloadResult {
                    server_type: server_id.server_type().to_string(),
                    bind_address: server_id.bind_address().to_string(),
                    loaded_at,
                });
            }
            Err(e) => {
                errors.push(CertificateReloadError {
                    server_type: server_id.server_type().to_string(),
                    bind_address: server_id.bind_address().to_string(),
                    error: e.to_string(),
                });
            }
        }
    }
    let status = if errors.is_empty() {
        "ok"
    } else if reloaded.is_empty() {
        "failed"
    } else {
        "partial"
    };
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": status,
        "reloaded": reloaded,
        "errors": errors
    }))
}

#[tracing::instrument(level = "debug")]
pub async fn api_service_certificate_status(
    request: HttpRequest,
    data: Data<Arc<ApiServiceData>>,
) -> HttpResponse {
    if let Some(error_return) = api_validation(&request, &data).await {
        return error_return;
    }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await {
        return response;
    }
    let certificate_store = &data.torrent_tracker.certificate_store;
    let certificates = certificate_store.get_all_certificates();
    let status_items: Vec<CertificateStatusItem> = certificates
        .into_iter()
        .map(|(server_id, bundle)| {
            CertificateStatusItem {
                server_type: server_id.server_type().to_string(),
                bind_address: server_id.bind_address().to_string(),
                cert_path: bundle.cert_path.clone(),
                key_path: bundle.key_path.clone(),
                loaded_at: bundle.loaded_at.to_rfc3339(),
            }
        })
        .collect();
    HttpResponse::Ok().content_type(ContentType::json()).json(json!({
        "status": "ok",
        "certificates": status_items
    }))
}