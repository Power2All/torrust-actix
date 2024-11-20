use std::sync::Arc;
use actix_web::{web, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use crate::api::api::{api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;

#[tracing::instrument(level = "debug")]
pub async fn api_service_stats_get(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    HttpResponse::Ok().content_type(ContentType::json()).json(data.torrent_tracker.get_stats())
}

#[tracing::instrument(level = "debug")]
pub async fn api_service_prom_get(request: HttpRequest, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Get stats
    let stats = data.torrent_tracker.get_stats();

    // Build Prometheus Output
    let mut string_output = vec![];

    string_output.extend(api_service_prom_generate_line("torrents", "Amount of torrents in memory", "gauge", stats.torrents));
    string_output.extend(api_service_prom_generate_line("torrents_updates", "Amount of torrents updates in memory", "gauge", stats.torrents_updates));
    string_output.extend(api_service_prom_generate_line("users", "Amount of torrents in memory", "gauge", stats.torrents));
    string_output.extend(api_service_prom_generate_line("users_updates", "Amount of users updates in memory", "gauge", stats.users_updates));
    string_output.extend(api_service_prom_generate_line("seeds", "Amount of seeds in memory", "gauge", stats.seeds));
    string_output.extend(api_service_prom_generate_line("peers", "Amount of peers in memory", "gauge", stats.peers));
    string_output.extend(api_service_prom_generate_line("completed", "Amount of completed in memory", "gauge", stats.completed));
    string_output.extend(api_service_prom_generate_line("whitelist", "Amount of whitelists in memory", "gauge", stats.whitelist));
    string_output.extend(api_service_prom_generate_line("whitelist_updates", "Amount of whitelists updates in memory", "gauge", stats.whitelist_updates));
    string_output.extend(api_service_prom_generate_line("blacklist", "Amount of blacklists in memory", "gauge", stats.blacklist));
    string_output.extend(api_service_prom_generate_line("blacklist_updates", "Amount of blacklists updates in memory", "gauge", stats.blacklist_updates));
    string_output.extend(api_service_prom_generate_line("key", "Amount of keys in memory", "gauge", stats.blacklist));
    string_output.extend(api_service_prom_generate_line("key_updates", "Amount of keys updates in memory", "gauge", stats.blacklist_updates));

    string_output.extend(api_service_prom_generate_line("tcp4_not_found", "Counter of IPv4 TCP Not Found (404)", "counter", stats.tcp4_not_found));
    string_output.extend(api_service_prom_generate_line("tcp4_failure", "Counter of IPv4 TCP Failure", "counter", stats.tcp4_failure));
    string_output.extend(api_service_prom_generate_line("tcp4_connections_handled", "Counter of IPv4 TCP Failure", "counter", stats.tcp4_connections_handled));
    string_output.extend(api_service_prom_generate_line("tcp4_api_handled", "Counter of IPv4 TCP API handled", "counter", stats.tcp4_api_handled));
    string_output.extend(api_service_prom_generate_line("tcp4_announces_handled", "Counter of IPv4 TCP Announces handled", "counter", stats.tcp4_announces_handled));
    string_output.extend(api_service_prom_generate_line("tcp4_scrapes_handled", "Counter of IPv4 TCP Scrapes handled", "counter", stats.tcp4_scrapes_handled));

    string_output.extend(api_service_prom_generate_line("tcp6_not_found", "Counter of IPv6 TCP Not Found (404)", "counter", stats.tcp6_not_found));
    string_output.extend(api_service_prom_generate_line("tcp6_failure", "Counter of IPv6 TCP Failure", "counter", stats.tcp6_failure));
    string_output.extend(api_service_prom_generate_line("tcp6_connections_handled", "Counter of IPv6 TCP Failure", "counter", stats.tcp6_connections_handled));
    string_output.extend(api_service_prom_generate_line("tcp6_api_handled", "Counter of IPv6 TCP API handled", "counter", stats.tcp6_api_handled));
    string_output.extend(api_service_prom_generate_line("tcp6_announces_handled", "Counter of IPv6 TCP Announces handled", "counter", stats.tcp6_announces_handled));
    string_output.extend(api_service_prom_generate_line("tcp6_scrapes_handled", "Counter of IPv6 TCP Scrapes handled", "counter", stats.tcp6_scrapes_handled));

    string_output.extend(api_service_prom_generate_line("udp4_bad_request", "Counter of IPv4 UDP Bad Request", "counter", stats.udp4_bad_request));
    string_output.extend(api_service_prom_generate_line("udp4_invalid_request", "Counter of IPv4 UDP Invalid Request", "counter", stats.udp4_invalid_request));
    string_output.extend(api_service_prom_generate_line("udp4_connections_handled", "Counter of IPv4 UDP Connections handled", "counter", stats.udp4_connections_handled));
    string_output.extend(api_service_prom_generate_line("udp4_announces_handled", "Counter of IPv4 UDP Announces handled", "counter", stats.udp4_announces_handled));
    string_output.extend(api_service_prom_generate_line("udp4_scrapes_handled", "Counter of IPv4 UDP Scrapes handled", "counter", stats.tcp4_scrapes_handled));
    string_output.extend(vec!["".to_string()]);

    HttpResponse::Ok().content_type(ContentType::plaintext()).body(string_output.join("\n"))
}

pub fn api_service_prom_generate_line(key: &str, description: &str, type_metric: &str, value: i64) -> Vec<String>
{
    vec![
        format!("# HELP {} {}", key, description).to_string(),
        format!("# TYPE {} {}", key, type_metric).to_string(),
        format!("{} {}", key, value).to_string(),
    ]
}