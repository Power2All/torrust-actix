use crate::api::api::{
    api_service_token,
    api_validation
};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_web::{
    web,
    HttpRequest,
    HttpResponse
};
use std::sync::Arc;

pub async fn api_service_stats_get(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    HttpResponse::Ok().content_type(ContentType::json()).json(data.torrent_tracker.get_stats())
}

pub async fn api_service_prom_get(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), Arc::clone(&data.torrent_tracker.config)).await { return response; }
    let stats = data.torrent_tracker.get_stats();
    let prometheus_id = &data.torrent_tracker.config.tracker_config.prometheus_id;
    let mut string_output = String::with_capacity(4096);
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "torrents", stats.torrents, true, Some(&format!("{prometheus_id} gauge metrics"))));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "torrents_updates", stats.torrents_updates, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "users", stats.users, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "users_updates", stats.users_updates, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "seeds", stats.seeds, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "peers", stats.peers, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "completed", stats.completed, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "whitelist", stats.whitelist, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "whitelist_updates", stats.whitelist_updates, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "blacklist", stats.blacklist, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "blacklist_updates", stats.blacklist_updates, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "keys", stats.keys, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "gauge", "keys_updates", stats.keys_updates, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp4_not_found", stats.tcp4_not_found, true, Some(&format!("{prometheus_id} counter metrics"))));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp4_failure", stats.tcp4_failure, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp4_connections_handled", stats.tcp4_connections_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp4_api_handled", stats.tcp4_api_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp4_announces_handled", stats.tcp4_announces_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp4_scrapes_handled", stats.tcp4_scrapes_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp6_not_found", stats.tcp6_not_found, true, Some(&format!("{prometheus_id} counter metrics"))));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp6_failure", stats.tcp6_failure, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp6_connections_handled", stats.tcp6_connections_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp6_api_handled", stats.tcp6_api_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp6_announces_handled", stats.tcp6_announces_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "tcp6_scrapes_handled", stats.tcp6_scrapes_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp4_bad_request", stats.udp4_bad_request, true, Some(&format!("{prometheus_id} counter metrics"))));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp4_invalid_request", stats.udp4_invalid_request, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp4_connections_handled", stats.udp4_connections_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp4_announces_handled", stats.udp4_announces_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp4_scrapes_handled", stats.udp4_scrapes_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp6_bad_request", stats.udp6_bad_request, true, Some(&format!("{prometheus_id} counter metrics"))));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp6_invalid_request", stats.udp6_invalid_request, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp6_connections_handled", stats.udp6_connections_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp6_announces_handled", stats.udp6_announces_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "udp6_scrapes_handled", stats.udp6_scrapes_handled, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_connections_active", stats.ws_connections_active, true, Some(&format!("{prometheus_id} WebSocket counter metrics"))));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_requests_sent", stats.ws_requests_sent, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_requests_received", stats.ws_requests_received, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_responses_sent", stats.ws_responses_sent, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_responses_received", stats.ws_responses_received, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_timeouts", stats.ws_timeouts, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_reconnects", stats.ws_reconnects, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_auth_success", stats.ws_auth_success, false, None));
    string_output.push_str(&api_service_prom_generate_line(prometheus_id, "counter", "ws_auth_failed", stats.ws_auth_failed, false, None));
    HttpResponse::Ok().content_type(ContentType::plaintext()).body(string_output)
}

pub fn api_service_prom_generate_line(id: &str, type_metric: &str, metric: &str, value: i64, without_header: bool, description: Option<&str>) -> String
{
    if without_header {
        format!(
            "# HELP {}_{} {}\n# TYPE {}_{} {}\n{}_{}{{metric=\"{}\"}} {}\n",
            id, type_metric, description.unwrap_or(""),
            id, type_metric, type_metric,
            id, type_metric, metric, value
        )
    } else {
        format!("{id}_{type_metric}{{metric=\"{metric}\"}} {value}\n")
    }
}