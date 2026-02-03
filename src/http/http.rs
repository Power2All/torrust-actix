use crate::common::common::parse_query;
use crate::common::structs::custom_error::CustomError;
use crate::config::enums::cluster_mode::ClusterMode;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::http::structs::http_service_data::HttpServiceData;
use crate::http::types::{HttpServiceQueryHashingMapErr, HttpServiceQueryHashingMapOk};
use crate::ssl::certificate_resolver::DynamicCertificateResolver;
use crate::ssl::certificate_store::ServerIdentifier;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_id::UserId;
use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::slave::forwarder::{create_cluster_error_response, forward_request};
use actix_cors::Cors;
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{Data, ServiceConfig};
use actix_web::{http, web, App, HttpRequest, HttpResponse, HttpServer};
use bip_bencode::{ben_bytes, ben_int, ben_list, ben_map, BMutAccess};
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::borrow::Cow;
use std::future::Future;
use std::io::Write;
use std::net::{IpAddr, SocketAddr};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;

lazy_static! {
    static ref ERR_MISSING_KEY: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("missing key") }.encode();
    static ref ERR_UNKNOWN_INFO_HASH: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("unknown info_hash") }.encode();
    static ref ERR_FORBIDDEN_INFO_HASH: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("forbidden info_hash") }.encode();
    static ref ERR_UNKNOWN_REQUEST: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("unknown request") }.encode();
    static ref ERR_UNABLE_DECODE_HEX: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("unable to decode hex string") }.encode();
    static ref ERR_UNKNOWN_ORIGIN_IP: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("unknown origin ip") }.encode();
    static ref ERR_INVALID_KEY: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("invalid key") }.encode();
    static ref ERR_UNKNOWN_KEY: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("unknown key") }.encode();
    static ref ERR_INVALID_USER_KEY: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("invalid user key") }.encode();
    static ref ERR_UNKNOWN_USER_KEY: Vec<u8> = ben_map!{ "failure reason" => ben_bytes!("unknown user key") }.encode();
}

#[tracing::instrument(level = "debug")]
pub fn http_service_cors() -> Cors
{
    Cors::default()
        .allow_any_origin()
        .send_wildcard()
        .allowed_methods(vec!["GET"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

#[tracing::instrument(level = "debug")]
pub fn http_service_routes(data: Arc<HttpServiceData>) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(data.clone()));
        cfg.service(web::resource("/announce")
            .route(web::get().to(http_service_announce))
        );
        cfg.service(web::resource("/{key}/announce")
            .route(web::get().to(http_service_announce_key))
        );
        cfg.service(web::resource("/{key}/{userkey}announce")
            .route(web::get().to(http_service_announce_userkey))
        );
        cfg.service(web::resource("/scrape")
            .route(web::get().to(http_service_scrape))
        );
        cfg.service(web::resource("/{key}/scrape")
            .route(web::get().to(http_service_scrape_key))
        );
        cfg.default_service(web::route().to(http_service_not_found));
    })
}

#[tracing::instrument(level = "debug")]
pub async fn http_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    http_server_object: HttpTrackersConfig,
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    let keep_alive = http_server_object.keep_alive;
    let request_timeout = http_server_object.request_timeout;
    let disconnect_timeout = http_server_object.disconnect_timeout;
    let worker_threads = http_server_object.threads as usize;
    if http_server_object.ssl {
        info!("[HTTPS] Starting server listener with SSL on {addr}");
        if http_server_object.ssl_key.is_empty() || http_server_object.ssl_cert.is_empty() {
            error!("[HTTPS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }
        let server_id = ServerIdentifier::HttpTracker(addr.to_string());
        if let Err(e) = data.certificate_store.load_certificate(
            server_id.clone(),
            &http_server_object.ssl_cert,
            &http_server_object.ssl_key,
        ) {
            panic!("[HTTPS] Failed to load SSL certificate: {}", e);
        }
        let resolver = match DynamicCertificateResolver::new(
            Arc::clone(&data.certificate_store),
            server_id,
        ) {
            Ok(resolver) => Arc::new(resolver),
            Err(e) => panic!("[HTTPS] Failed to create certificate resolver: {}", e),
        };
        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_cert_resolver(resolver);
        let service_data = Arc::new(HttpServiceData {
            torrent_tracker: data.clone(),
            http_trackers_config: Arc::new(http_server_object.clone())
        });
        let sentry_enabled = data.config.sentry_config.enabled;
        let server = if sentry_enabled {
            HttpServer::new(move || {
                App::new()
                    .wrap(sentry_actix::Sentry::new())
                    .wrap(http_service_cors())
                    .configure(http_service_routes(service_data.clone()))
            })
                .keep_alive(Duration::from_secs(keep_alive))
                .client_request_timeout(Duration::from_secs(request_timeout))
                .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
                .workers(worker_threads)
                .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
                .unwrap()
                .disable_signals()
                .run()
        } else {
            HttpServer::new(move || {
                App::new()
                    .wrap(http_service_cors())
                    .configure(http_service_routes(service_data.clone()))
            })
                .keep_alive(Duration::from_secs(keep_alive))
                .client_request_timeout(Duration::from_secs(request_timeout))
                .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
                .workers(worker_threads)
                .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
                .unwrap()
                .disable_signals()
                .run()
        };
        return (server.handle(), server);
    }
    info!("[HTTP] Starting server listener on {addr}");
    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: data.clone(),
        http_trackers_config: Arc::new(http_server_object.clone())
    });
    let sentry_enabled = data.config.sentry_config.enabled;
    let server = if sentry_enabled {
        HttpServer::new(move || {
            App::new()
                .wrap(sentry_actix::Sentry::new())
                .wrap(http_service_cors())
                .configure(http_service_routes(service_data.clone()))
        })
            .keep_alive(Duration::from_secs(keep_alive))
            .client_request_timeout(Duration::from_secs(request_timeout))
            .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
            .workers(worker_threads)
            .bind((addr.ip(), addr.port()))
            .unwrap()
            .disable_signals()
            .run()
    } else {
        HttpServer::new(move || {
            App::new()
                .wrap(http_service_cors())
                .configure(http_service_routes(service_data.clone()))
        })
            .keep_alive(Duration::from_secs(keep_alive))
            .client_request_timeout(Duration::from_secs(request_timeout))
            .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
            .workers(worker_threads)
            .bind((addr.ip(), addr.port()))
            .unwrap()
            .disable_signals()
            .run()
    };
    (server.handle(), server)
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_announce_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            http_stat_update(ip, &data.torrent_tracker, StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => { return result; }
    };
    let tracker_config = &data.torrent_tracker.config.tracker_config;
    if tracker_config.keys_enabled {
        let key = path.clone();
        let key_check = http_service_check_key_validation(data.torrent_tracker.clone(), key).await;
        if let Some(value) = key_check {
            http_stat_update(ip, &data.torrent_tracker, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return value;
        }
    }
    if tracker_config.users_enabled && !tracker_config.keys_enabled {
        let user_key = path.clone();
        let user_key_check = http_service_check_user_key_validation(data.torrent_tracker.clone(), user_key.clone()).await;
        if user_key_check.is_none() {
            return http_service_announce_handler(request, ip, data.torrent_tracker.clone(), Some(http_service_decode_hex_user_id(user_key.clone()).await.unwrap())).await;
        }
    }
    http_service_announce_handler(request, ip, data.torrent_tracker.clone(), None).await
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_announce_userkey(request: HttpRequest, path: web::Path<(String, String)>, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            http_stat_update(ip, &data.torrent_tracker, StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => { return result; }
    };
    let tracker_config = &data.torrent_tracker.config.tracker_config;
    if tracker_config.keys_enabled {
        let key = path.clone().0;
        let key_check = http_service_check_key_validation(data.torrent_tracker.clone(), key).await;
        if let Some(value) = key_check {
            http_stat_update(ip, &data.torrent_tracker, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return value;
        }
    }
    if tracker_config.users_enabled {
        let user_key = path.clone().1;
        let user_key_check = http_service_check_user_key_validation(data.torrent_tracker.clone(), user_key.clone()).await;
        if user_key_check.is_none() {
            return http_service_announce_handler(request, ip, data.torrent_tracker.clone(), Some(http_service_decode_hex_user_id(user_key.clone()).await.unwrap())).await;
        }
    }
    http_service_announce_handler(request, ip, data.torrent_tracker.clone(), None).await
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_announce(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            http_stat_update(ip, &data.torrent_tracker, StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => {
            return result;
        }
    };
    if data.torrent_tracker.config.tracker_config.keys_enabled {
        http_stat_update(ip, &data.torrent_tracker, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_MISSING_KEY.clone());
    }
    http_service_announce_handler(request, ip, data.torrent_tracker.clone(), None).await
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_announce_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>, user_key: Option<UserId>) -> HttpResponse
{
    if data.config.tracker_config.cluster == ClusterMode::slave {
        let query_string = request.query_string().to_string();
        let protocol = if request.connection_info().scheme() == "https" {
            ProtocolType::Https
        } else {
            ProtocolType::Http
        };
        let client_port = request.peer_addr().map(|a| a.port()).unwrap_or(0);
        match forward_request(
            &data,
            protocol,
            RequestType::Announce,
            ip,
            client_port,
            query_string.into_bytes(),
        ).await {
            Ok(response) => {
                return HttpResponse::Ok()
                    .content_type(ContentType::plaintext())
                    .body(response.payload);
            }
            Err(e) => {
                http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
                return HttpResponse::Ok()
                    .content_type(ContentType::plaintext())
                    .body(create_cluster_error_response(&e));
            }
        }
    }
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => {
            http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return err;
        }
    };
    let announce = data.validate_announce(ip, query_map).await;
    let announce_unwrapped = match announce {
        Ok(result) => { result }
        Err(e) => {
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode());
        }
    };
    let tracker_config = &data.config.tracker_config;
    if tracker_config.whitelist_enabled && !data.check_whitelist(announce_unwrapped.info_hash) {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_INFO_HASH.clone());
    }
    if tracker_config.blacklist_enabled && data.check_blacklist(announce_unwrapped.info_hash) {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_FORBIDDEN_INFO_HASH.clone());
    }
    let (_torrent_peer, torrent_entry) = match data.handle_announce(data.clone(), announce_unwrapped.clone(), user_key).await {
        Ok(result) => { result }
        Err(e) => {
            http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode());
        }
    };
    let request_interval = tracker_config.request_interval as i64;
    let request_interval_minimum = tracker_config.request_interval_minimum as i64;
    let seeds_count = torrent_entry.seeds.len() as i64;
    let peers_count = torrent_entry.peers.len() as i64;
    let completed_count = torrent_entry.completed as i64;
    if announce_unwrapped.compact {
        let mut peers_list: Vec<u8> = Vec::with_capacity(72 * 6);
        let port_bytes = announce_unwrapped.port.to_be_bytes();
        return match ip {
            IpAddr::V4(_) => {
                if announce_unwrapped.left != 0 {
                    let seeds = data.get_peers(
                        &torrent_entry.seeds,
                        TorrentPeersType::IPv4,
                        Some(ip),
                        72
                    );
                    for (_, torrent_peer) in seeds.iter() {
                        
                        if let IpAddr::V4(ipv4) = torrent_peer.peer_addr.ip() {
                            let _ = peers_list.write(&ipv4.octets());
                            let _ = peers_list.write(&port_bytes);
                        }
                    }
                }
                if peers_list.len() < 72 * 6 {
                    let peers = data.get_peers(
                        &torrent_entry.peers,
                        TorrentPeersType::IPv4,
                        Some(ip),
                        72
                    );
                    for (_, torrent_peer) in peers.iter() {
                        if peers_list.len() >= 72 * 6 {
                            break;
                        }
                        
                        if let IpAddr::V4(ipv4) = torrent_peer.peer_addr.ip() {
                            let _ = peers_list.write(&ipv4.octets());
                            let _ = peers_list.write(&port_bytes);
                        }
                    }
                }
                HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(request_interval),
                    "min interval" => ben_int!(request_interval_minimum),
                    "complete" => ben_int!(seeds_count),
                    "incomplete" => ben_int!(peers_count),
                    "downloaded" => ben_int!(completed_count),
                    "peers" => ben_bytes!(peers_list)
                }.encode())
            }
            IpAddr::V6(_) => {
                if announce_unwrapped.left != 0 {
                    let seeds = data.get_peers(
                        &torrent_entry.seeds,
                        TorrentPeersType::IPv6,
                        Some(ip),
                        72
                    );
                    for (_, torrent_peer) in seeds.iter() {
                        
                        if let IpAddr::V6(ipv6) = torrent_peer.peer_addr.ip() {
                            let _ = peers_list.write(&ipv6.octets());
                            let _ = peers_list.write(&port_bytes);
                        }
                    }
                }
                if peers_list.len() < 72 * 18 {
                    let peers = data.get_peers(
                        &torrent_entry.peers,
                        TorrentPeersType::IPv6,
                        Some(ip),
                        72
                    );
                    for (_, torrent_peer) in peers.iter() {
                        if peers_list.len() >= 72 * 18 {
                            break;
                        }
                        
                        if let IpAddr::V6(ipv6) = torrent_peer.peer_addr.ip() {
                            let _ = peers_list.write(&ipv6.octets());
                            let _ = peers_list.write(&port_bytes);
                        }
                    }
                }
                HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(request_interval),
                    "min interval" => ben_int!(request_interval_minimum),
                    "complete" => ben_int!(seeds_count),
                    "incomplete" => ben_int!(peers_count),
                    "downloaded" => ben_int!(completed_count),
                    "peers6" => ben_bytes!(peers_list)
                }.encode())
            }
        }
    }
    let mut peers_list = ben_list!();
    let peers_list_mut = peers_list.list_mut().unwrap();
    match ip {
        IpAddr::V4(_) => {
            if announce_unwrapped.left != 0 {
                let seeds = data.get_peers(
                    &torrent_entry.seeds,
                    TorrentPeersType::IPv4,
                    Some(ip),
                    72
                );
                for (peer_id, torrent_peer) in seeds.iter() {
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            if peers_list_mut.len() < 72 {
                let peers = data.get_peers(
                    &torrent_entry.peers,
                    TorrentPeersType::IPv4,
                    Some(ip),
                    72
                );
                for (peer_id, torrent_peer) in peers.iter() {
                    if peers_list_mut.len() >= 72 {
                        break;
                    }
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(request_interval),
                "min interval" => ben_int!(request_interval_minimum),
                "complete" => ben_int!(seeds_count),
                "incomplete" => ben_int!(peers_count),
                "downloaded" => ben_int!(completed_count),
                "peers" => peers_list
            }.encode())
        }
        IpAddr::V6(_) => {
            if announce_unwrapped.left != 0 {
                let seeds = data.get_peers(
                    &torrent_entry.seeds,
                    TorrentPeersType::IPv6,
                    Some(ip),
                    72
                );
                for (peer_id, torrent_peer) in seeds.iter() {
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            if peers_list_mut.len() < 72 {
                let peers = data.get_peers(
                    &torrent_entry.peers,
                    TorrentPeersType::IPv6,
                    Some(ip),
                    72
                );
                for (peer_id, torrent_peer) in peers.iter() {
                    if peers_list_mut.len() >= 72 {
                        break;
                    }
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                }
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(request_interval),
                "min interval" => ben_int!(request_interval_minimum),
                "complete" => ben_int!(seeds_count),
                "incomplete" => ben_int!(peers_count),
                "downloaded" => ben_int!(completed_count),
                "peers6" => peers_list
            }.encode())
        }
    }
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_scrape_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            if ip.is_ipv4() {
                data.torrent_tracker.update_stats(StatsEvent::Tcp4ScrapesHandled, 1);
            } else {
                data.torrent_tracker.update_stats(StatsEvent::Tcp6ScrapesHandled, 1);
            }
            ip
        },
        Err(result) => {
            return result;
        }
    };
    debug!("[DEBUG] Request from {ip}: Scrape with Key");
    if data.torrent_tracker.config.tracker_config.keys_enabled {
        let key = path.into_inner();
        let key_check = http_service_check_key_validation(data.torrent_tracker.clone(), key).await;
        if let Some(value) = key_check { return value; }
    }
    http_service_scrape_handler(request, ip, data.torrent_tracker.clone()).await
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_scrape_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>) -> HttpResponse
{
    if data.config.tracker_config.cluster == ClusterMode::slave {
        let query_string = request.query_string().to_string();
        let protocol = if request.connection_info().scheme() == "https" {
            ProtocolType::Https
        } else {
            ProtocolType::Http
        };
        let client_port = request.peer_addr().map(|a| a.port()).unwrap_or(0);
        match forward_request(
            &data,
            protocol,
            RequestType::Scrape,
            ip,
            client_port,
            query_string.into_bytes(),
        ).await {
            Ok(response) => {
                return HttpResponse::Ok()
                    .content_type(ContentType::plaintext())
                    .body(response.payload);
            }
            Err(e) => {
                http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
                return HttpResponse::Ok()
                    .content_type(ContentType::plaintext())
                    .body(create_cluster_error_response(&e));
            }
        }
    }
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => {
            http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return err;
        }
    };
    let scrape = data.validate_scrape(query_map).await;
    if let Err(scrape) = scrape {
        http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "failure reason" => ben_bytes!(scrape.to_string())
        }.encode());
    }
    let tracker_config = &data.config.tracker_config;
    let request_interval = tracker_config.request_interval as i64;
    let request_interval_minimum = tracker_config.request_interval_minimum as i64;
    match scrape.as_ref() {
        Ok(e) => {
            let data_scrape = data.handle_scrape(data.clone(), e.clone()).await;
            let mut scrape_list = ben_map!();
            let scrape_list_mut = scrape_list.dict_mut().unwrap();
            for (info_hash, torrent_entry) in data_scrape.iter() {
                scrape_list_mut.insert(Cow::from(info_hash.0.to_vec()), ben_map! {
                    "complete" => ben_int!(torrent_entry.seeds.len() as i64),
                    "downloaded" => ben_int!(torrent_entry.completed as i64),
                    "incomplete" => ben_int!(torrent_entry.peers.len() as i64)
                });
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(request_interval),
                "min interval" => ben_int!(request_interval_minimum),
                "files" => scrape_list
            }.encode())
        }
        Err(e) => {
            http_stat_update(ip, &data, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode())
        }
    }
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_scrape(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            if ip.is_ipv4() {
                data.torrent_tracker.update_stats(StatsEvent::Tcp4ScrapesHandled, 1);
            } else {
                data.torrent_tracker.update_stats(StatsEvent::Tcp6ScrapesHandled, 1);
            }
            ip
        },
        Err(result) => {
            return result;
        }
    };
    debug!("[DEBUG] Request from {ip}: Scrape");
    http_service_scrape_handler(request, ip, data.torrent_tracker.clone()).await
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_not_found(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            if ip.is_ipv4() {
                data.torrent_tracker.update_stats(StatsEvent::Tcp4NotFound, 1);
            } else {
                data.torrent_tracker.update_stats(StatsEvent::Tcp6NotFound, 1);
            }
            ip
        },
        Err(result) => {
            return result;
        }
    };
    debug!("[DEBUG] Request from {ip}: 404 Not Found");
    HttpResponse::NotFound().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_REQUEST.clone())
}

#[tracing::instrument(level = "debug")]
#[inline]
pub fn http_service_stats_log(ip: IpAddr, tracker: &TorrentTracker)
{
    if ip.is_ipv4() {
        tracker.update_stats(StatsEvent::Tcp4ConnectionsHandled, 1);
    } else {
        tracker.update_stats(StatsEvent::Tcp6ConnectionsHandled, 1);
    }
}

#[tracing::instrument(level = "debug")]
#[inline]
pub async fn http_service_decode_hex_hash(hash: String) -> Result<InfoHash, HttpResponse>
{
    hex::decode(&hash)
        .ok()
        .and_then(|bytes| bytes.get(..20).and_then(|slice| <[u8; 20]>::try_from(slice).ok()))
        .map(InfoHash)
        .ok_or_else(|| HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(ERR_UNABLE_DECODE_HEX.clone()))
}

#[tracing::instrument(level = "debug")]
#[inline]
pub async fn http_service_decode_hex_user_id(hash: String) -> Result<UserId, HttpResponse>
{
    hex::decode(&hash)
        .ok()
        .and_then(|bytes| bytes.get(..20).and_then(|slice| <[u8; 20]>::try_from(slice).ok()))
        .map(UserId)
        .ok_or_else(|| HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(ERR_UNABLE_DECODE_HEX.clone()))
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_retrieve_remote_ip(request: HttpRequest, data: Arc<HttpTrackersConfig>) -> Result<IpAddr, ()>
{
    let origin_ip = request.peer_addr().map(|addr| addr.ip()).ok_or(())?;
    request.headers()
        .get(&data.real_ip)
        .and_then(|header| header.to_str().ok())
        .and_then(|ip_str| IpAddr::from_str(ip_str).ok())
        .map(Ok)
        .unwrap_or(Ok(origin_ip))
}

#[tracing::instrument(level = "debug")]
pub async fn http_validate_ip(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> Result<IpAddr, HttpResponse>
{
    match http_service_retrieve_remote_ip(request.clone(), data.http_trackers_config.clone()).await {
        Ok(ip) => {
            http_service_stats_log(ip, &data.torrent_tracker);
            Ok(ip)
        }
        Err(_) => {
            Err(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_ORIGIN_IP.clone()))
        }
    }
}

#[tracing::instrument(level = "debug")]
pub fn http_service_query_hashing(query_map_result: Result<HttpServiceQueryHashingMapOk, CustomError>) -> Result<HttpServiceQueryHashingMapOk, HttpServiceQueryHashingMapErr>
{
    match query_map_result {
        Ok(e) => { Ok(e) }
        Err(e) => {
            Err(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode()))
        }
    }
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_check_key_validation(data: Arc<TorrentTracker>, key: String) -> Option<HttpResponse>
{
    if key.len() != 40 {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_INVALID_KEY.clone()));
    }
    let key_decoded: InfoHash = match http_service_decode_hex_hash(key).await {
        Ok(result) => { result }
        Err(error) => { return Some(error) }
    };
    if !data.check_key(key_decoded) {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_KEY.clone()));
    }
    None
}

#[tracing::instrument(level = "debug")]
pub async fn http_service_check_user_key_validation(data: Arc<TorrentTracker>, user_key: String) -> Option<HttpResponse>
{
    if user_key.len() != 40 {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_INVALID_USER_KEY.clone()));
    }
    let user_key_decoded: UserId = match http_service_decode_hex_user_id(user_key).await {
        Ok(result) => { result }
        Err(error) => { return Some(error) }
    };
    if data.check_user_key(user_key_decoded).is_none() {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_USER_KEY.clone()));
    }
    None
}

#[tracing::instrument(level = "debug")]
pub fn http_check_host_and_port_used(bind_address: String) {
    if cfg!(target_os = "windows") {
        match std::net::TcpListener::bind(&bind_address) {
            Ok(e) => e,
            Err(_) => { panic!("Unable to bind to {} ! Exiting...", &bind_address); }
        };
    }
}

#[tracing::instrument(level = "debug")]
#[inline]
pub fn http_stat_update(ip: IpAddr, data: &TorrentTracker, stats_ipv4: StatsEvent, stat_ipv6: StatsEvent, count: i64)
{
    match ip {
        IpAddr::V4(_) => {
            data.update_stats(stats_ipv4, count);
        }
        IpAddr::V6(_) => {
            data.update_stats(stat_ipv6, count);
        }
    }
}