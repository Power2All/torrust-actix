use std::borrow::Cow;
use std::fs::File;
use std::future::Future;
use std::io::{BufReader, Write};
use std::net::{IpAddr, SocketAddr};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use actix_cors::Cors;
use actix_web::{App, http, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{Data, ServiceConfig};
use bip_bencode::{ben_bytes, ben_int, ben_list, ben_map, BMutAccess};
use log::{debug, error, info};
use crate::common::common::{maintenance_mode, parse_query};
use crate::common::structs::custom_error::CustomError;
use crate::http::types::{HttpServiceQueryHashingMapErr, HttpServiceQueryHashingMapOk};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_id::UserId;

pub fn http_service_cors() -> Cors
{
    Cors::default()
        .send_wildcard()
        .allowed_methods(vec!["GET"])
        .allowed_headers(vec![http::header::X_FORWARDED_FOR, http::header::ACCEPT])
        .allowed_header(http::header::CONTENT_TYPE)
        .max_age(1)
}

pub fn http_service_routes(data: Arc<TorrentTracker>) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(web::Data::new(data.clone()));
        // cfg.service(web::resource("/webtorrent").route(web::get().to(websocket_service_announce)));
        // cfg.service(web::resource("/webtorrent/{key}").route(web::get().to(websocket_service_announce_key)));
        cfg.service(web::resource("/announce").route(web::get().to(http_service_announce)));
        cfg.service(web::resource("/announce/{key}").route(web::get().to(http_service_announce_key)));
        cfg.service(web::resource("/announce/{key}/{userkey}").route(web::get().to(http_service_announce_userkey)));
        cfg.service(web::resource("/scrape").route(web::get().to(http_service_scrape)));
        cfg.service(web::resource("/scrape/{key}").route(web::get().to(http_service_scrape_key)));
        cfg.default_service(web::route().to(http_service_not_found));
    })
}

pub async fn http_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    keep_alive: u64,
    client_request_timeout: u64,
    client_disconnect_timeout: u64,
    threads: u64,
    ssl: (bool, Option<String>, Option<String>) /* 0: ssl enabled, 1: key, 2: cert */
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    if ssl.0 {
        info!("[HTTP] Starting server listener with SSL on {}", addr);
        if ssl.1.is_none() || ssl.2.is_none() {
            error!("[HTTP] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }

        let key_file = &mut BufReader::new(File::open(ssl.1.clone().unwrap()).unwrap());
        let certs_file = &mut BufReader::new(File::open(ssl.2.clone().unwrap()).unwrap());

        let tls_certs = rustls_pemfile::certs(certs_file)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();
        let tls_key = match rustls_pemfile::pkcs8_private_keys(key_file).next().unwrap() {
            Err(_) => {
                exit(1);
            }
            Ok(data) => {
                data
            }
        };

        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .unwrap();

        let server = HttpServer::new(move || {
            App::new()
                .wrap(http_service_cors())
                .configure(http_service_routes(data.clone()))
        })
            .keep_alive(Duration::from_secs(keep_alive))
            .client_request_timeout(Duration::from_secs(client_request_timeout))
            .client_disconnect_timeout(Duration::from_secs(client_disconnect_timeout))
            .workers(threads as usize)
            .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
            .unwrap()
            .disable_signals()
            .run();

        return (server.handle(), server);
    }

    info!("[HTTP] Starting server listener on {}", addr);
    let server = HttpServer::new(move || {
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(data.clone()))
    })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(client_request_timeout))
        .client_disconnect_timeout(Duration::from_secs(client_disconnect_timeout))
        .workers(threads as usize)
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();

    (server.handle(), server)
}

pub async fn http_service_announce_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<TorrentTracker>>) -> HttpResponse
{
    data.update_stats(StatsEvent::TestCounter, 1).await;
    let stat_test_counter = data.get_stats().await.test_counter;
    let start = Instant::now();
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        data.set_stats(StatsEvent::TestCounter, 0).await;
    }

    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce_key: {:?}", start.elapsed());
            }
            return result;
        }
    };

    if ip.is_ipv4() {
        data.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1).await;
    } else {
        data.update_stats(StatsEvent::Tcp6AnnouncesHandled, 1).await;
    }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await {
        if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
            info!("[PERF] http_service_announce_key: {:?}", start.elapsed());
        }
        return result;
    }

    if data.config.keys {
        let key = path.clone();
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(value) = key_check {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce_key: {:?}", start.elapsed());
            }
            return value;
        }
    }

    if data.config.users && !data.config.keys {
        let user_key = path.clone();
        let user_key_check = http_service_check_user_key_validation(data.as_ref().clone(), user_key.clone()).await;
        if user_key_check.is_none() {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce_key: {:?}", start.elapsed());
            }
            return http_service_announce_handler(request, ip, data.as_ref().clone(), Some(http_service_decode_hex_user_id(user_key.clone()).await.unwrap())).await;
        }
    }

    let response = http_service_announce_handler(request, ip, data.as_ref().clone(), None).await;
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        info!("[PERF] http_service_announce_key: {:?}", start.elapsed());
    }
    response
}

pub async fn http_service_announce_userkey(request: HttpRequest, path: web::Path<(String, String)>, data: Data<Arc<TorrentTracker>>) -> HttpResponse
{
    data.update_stats(StatsEvent::TestCounter, 1).await;
    let stat_test_counter = data.get_stats().await.test_counter;
    let start = Instant::now();
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        data.set_stats(StatsEvent::TestCounter, 0).await;
    }

    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce_userkey: {:?}", start.elapsed());
            }
            return result;
        }
    };

    if ip.is_ipv4() {
        data.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1).await;
    } else {
        data.update_stats(StatsEvent::Tcp6AnnouncesHandled, 1).await;
    }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await {
        if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
            info!("[PERF] http_service_announce_userkey: {:?}", start.elapsed());
        }
        return result;
    }

    if data.config.keys {
        let key = path.clone().0;
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(value) = key_check {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce_userkey: {:?}", start.elapsed());
            }
            return value;
        }
    }

    if data.config.users {
        let user_key = path.clone().1;
        let user_key_check = http_service_check_user_key_validation(data.as_ref().clone(), user_key.clone()).await;
        if user_key_check.is_none() {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce_userkey: {:?}", start.elapsed());
            }
            return http_service_announce_handler(request, ip, data.as_ref().clone(), Some(http_service_decode_hex_user_id(user_key.clone()).await.unwrap())).await;
        }
    }

    let response = http_service_announce_handler(request, ip, data.as_ref().clone(), None).await;
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        info!("[PERF] http_service_announce_userkey: {:?}", start.elapsed());
    }
    response
}

pub async fn http_service_announce(request: HttpRequest, data: Data<Arc<TorrentTracker>>) -> HttpResponse
{
    data.update_stats(StatsEvent::TestCounter, 1).await;
    let stat_test_counter = data.get_stats().await.test_counter;
    let start = Instant::now();
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        data.set_stats(StatsEvent::TestCounter, 0).await;
    }

    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_announce: {:?}", start.elapsed());
            }
            return result;
        }
    };

    if ip.is_ipv4() {
        data.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1).await;
    } else {
        data.update_stats(StatsEvent::Tcp6AnnouncesHandled, 1).await;
    }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await {
        if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
            info!("[PERF] http_service_announce: {:?}", start.elapsed());
        }
        return result;
    }

    if data.config.keys {
        if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
            info!("[PERF] http_service_announce: {:?}", start.elapsed());
        }
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("missing key")
            }.encode());
    }

    let response = http_service_announce_handler(request, ip, data.as_ref().clone(), None).await;
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        info!("[PERF] http_service_announce: {:?}", start.elapsed());
    }
    response
}

pub async fn http_service_announce_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>, user_key: Option<UserId>) -> HttpResponse
{
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => { return err; }
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

    if data.config.whitelist && !data.check_whitelist(announce_unwrapped.info_hash).await {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("unknown info_hash")
            }.encode());
    }

    if data.config.blacklist && data.check_blacklist(announce_unwrapped.info_hash).await {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("forbidden info_hash")
            }.encode());
    }

    let (_torrent_peer, torrent_entry) = match data.handle_announce(data.clone(), announce_unwrapped.clone(), user_key).await {
        Ok(result) => { result }
        Err(e) => {
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "failure reason" => ben_bytes!(e.to_string())
                }.encode());
        }
    };

    let mut peer_count = 0;
    if announce_unwrapped.clone().compact {
        let mut peers: Vec<u8> = Vec::new();
        if announce_unwrapped.clone().left != 0 {
            for (_peer_id, torrent_peer) in torrent_entry.seeds.iter() {
                if peer_count == data.config.peers_returned.unwrap_or(72) {
                    break;
                }
                let _ = match torrent_peer.peer_addr.ip() {
                    IpAddr::V4(ip) => {
                        peers.write(&u32::from(ip).to_be_bytes())
                    },
                    IpAddr::V6(ip) => {
                        peers.write(&u128::from(ip).to_be_bytes())
                    }
                };
                peers.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
                peer_count += 1;
            }
        }
        for (_peer_id, torrent_peer) in torrent_entry.peers.iter() {
            if peer_count == data.config.peers_returned.unwrap_or(72) {
                break;
            }
            let _ = match torrent_peer.peer_addr.ip() {
                IpAddr::V4(ip) => {
                    peers.write(&u32::from(ip).to_be_bytes())
                },
                IpAddr::V6(ip) => {
                    peers.write(&u128::from(ip).to_be_bytes())
                }
            };
            peers.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
            peer_count += 1;
        }
        return if announce_unwrapped.clone().remote_addr.is_ipv4() {
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(data.config.interval.unwrap() as i64),
                    "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                    "complete" => ben_int!(torrent_entry.seeds_count as i64),
                    "incomplete" => ben_int!(torrent_entry.peers_count as i64),
                    "downloaded" => ben_int!(torrent_entry.completed),
                    "peers" => ben_bytes!(peers)
                }.encode())
        } else {
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(data.config.interval.unwrap() as i64),
                    "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                    "complete" => ben_int!(torrent_entry.seeds_count as i64),
                    "incomplete" => ben_int!(torrent_entry.peers_count as i64),
                    "downloaded" => ben_int!(torrent_entry.completed),
                    "peers6" => ben_bytes!(peers)
                }.encode())
        };
    }

    let mut peers_list = ben_list!();
    let peers_list_mut = peers_list.list_mut().unwrap();
    for (peer_id, torrent_peer) in torrent_entry.peers.iter() {
        if peer_count == data.config.peers_returned.unwrap_or(72) {
            break;
        }
        match torrent_peer.peer_addr.ip() {
            IpAddr::V4(_) => {
                if announce_unwrapped.clone().remote_addr.is_ipv4() {
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.clone().to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                    peer_count += 1;
                }
            },
            IpAddr::V6(_) => {
                if announce_unwrapped.clone().remote_addr.is_ipv6() {
                    peers_list_mut.push(ben_map! {
                        "peer id" => ben_bytes!(peer_id.clone().to_string()),
                        "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                        "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                    });
                    peer_count += 1;
                }
            }
        }
    }
    if announce_unwrapped.clone().remote_addr.is_ipv4() {
        HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(data.config.interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                "complete" => ben_int!(torrent_entry.seeds_count as i64),
                "incomplete" => ben_int!(torrent_entry.peers_count as i64),
                "downloaded" => ben_int!(torrent_entry.completed),
                "peers" => peers_list
            }.encode())
    } else {
        HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(data.config.interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                "complete" => ben_int!(torrent_entry.seeds_count as i64),
                "incomplete" => ben_int!(torrent_entry.peers_count as i64),
                "downloaded" => ben_int!(torrent_entry.completed),
                "peers6" => peers_list
            }.encode())
    }
}

pub async fn http_service_scrape_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<TorrentTracker>>) -> HttpResponse
{
    data.update_stats(StatsEvent::TestCounter, 1).await;
    let stat_test_counter = data.get_stats().await.test_counter;
    let start = Instant::now();
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        data.set_stats(StatsEvent::TestCounter, 0).await;
    }

    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_scrape_key: {:?}", start.elapsed());
            }
            return result;
        }
    };

    debug!("[DEBUG] Request from {}: Scrape with Key", ip);

    if ip.is_ipv4() {
        data.update_stats(StatsEvent::Tcp4ScrapesHandled, 1).await;
    } else {
        data.update_stats(StatsEvent::Tcp6ScrapesHandled, 1).await;
    }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await {
        if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
            info!("[PERF] http_service_scrape_key: {:?}", start.elapsed());
        }
        return result;
    }

    if data.config.keys {
        let key = path.into_inner();
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(value) = key_check {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_scrape_key: {:?}", start.elapsed());
            }
            return value;
        }
    }

    let response = http_service_scrape_handler(request, data.as_ref().clone()).await;
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        info!("[PERF] http_service_scrape_key: {:?}", start.elapsed());
    }
    response
}

pub async fn http_service_scrape_handler(request: HttpRequest, data: Arc<TorrentTracker>) -> HttpResponse
{
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let scrape = data.validate_scrape(query_map).await;
    if scrape.is_err() {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(scrape.unwrap_err().to_string())
            }.encode());
    }

    match scrape.as_ref() {
        Ok(e) => {
            let data_scrape = data.handle_scrape(data.clone(), e.clone()).await;
            let mut scrape_list = ben_map!();
            let scrape_list_mut = scrape_list.dict_mut().unwrap();
            for (key, value) in data_scrape.iter() {
                scrape_list_mut.insert(Cow::from(key.0.to_vec()), ben_map! {
                    "complete" => ben_int!(value.seeds_count as i64),
                    "downloaded" => ben_int!(value.completed),
                    "incomplete" => ben_int!(value.peers_count as i64)
                });
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(data.config.interval.unwrap() as i64),
                    "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                    "files" => scrape_list
                }.encode())
        }
        Err(e) => {
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "failure reason" => ben_bytes!(e.to_string())
                }.encode())
        }
    }
}

pub async fn http_service_scrape(request: HttpRequest, data: Data<Arc<TorrentTracker>>) -> HttpResponse
{
    data.update_stats(StatsEvent::TestCounter, 1).await;
    let stat_test_counter = data.get_stats().await.test_counter;
    let start = Instant::now();
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        data.set_stats(StatsEvent::TestCounter, 0).await;
    }

    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_scrape: {:?}", start.elapsed());
            }
            return result;
        }
    };

    debug!("[DEBUG] Request from {}: Scrape", ip);

    if ip.is_ipv4() {
        data.update_stats(StatsEvent::Tcp4ScrapesHandled, 1).await;
    } else {
        data.update_stats(StatsEvent::Tcp6ScrapesHandled, 1).await;
    }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await {
        if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
            info!("[PERF] http_service_scrape: {:?}", start.elapsed());
        }
        return result;
    }

    let response = http_service_scrape_handler(request, data.as_ref().clone()).await;
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        info!("[PERF] http_service_scrape: {:?}", start.elapsed());
    }
    response
}

pub async fn http_service_not_found(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    data.update_stats(StatsEvent::TestCounter, 1).await;
    let stat_test_counter = data.get_stats().await.test_counter;
    let start = Instant::now();
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        data.set_stats(StatsEvent::TestCounter, 0).await;
    }

    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => ip,
        Err(result) => {
            if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
                info!("[PERF] http_service_not_found: {:?}", start.elapsed());
            }
            return result;
        }
    };

    debug!("[DEBUG] Request from {}: 404 Not Found", ip);

    let response = HttpResponse::NotFound().content_type(ContentType::plaintext()).body(std::str::from_utf8(&ben_map! {
            "failure reason" => ben_bytes!("unknown request")
        }.encode()).unwrap().to_string());
    if stat_test_counter > data.config.log_perf_count.unwrap_or(10000) as i64 {
        info!("[PERF] http_service_not_found: {:?}", start.elapsed());
    }
    response
}

pub async fn http_service_stats_log(ip: IpAddr, tracker: web::Data<Arc<TorrentTracker>>)
{
    if ip.is_ipv4() {
        tracker.update_stats(StatsEvent::Tcp4ConnectionsHandled, 1).await;
    } else {
        tracker.update_stats(StatsEvent::Tcp6ConnectionsHandled, 1).await;
    }
}

pub async fn http_service_decode_hex_hash(hash: String) -> Result<InfoHash, HttpResponse>
{
    return match hex::decode(hash) {
        Ok(hash_result) => {
            Ok(InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()))
        }
        Err(_) => {
            return Err(HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(std::str::from_utf8(&ben_map! {
                    "failure reason" => ben_bytes!("unable to decode hex string")
                }.encode()).unwrap().to_string()));
        }
    };
}

pub async fn http_service_decode_hex_user_id(hash: String) -> Result<UserId, HttpResponse>
{
    return match hex::decode(hash) {
        Ok(hash_result) => {
            Ok(UserId(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap()))
        }
        Err(_) => {
            return Err(HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(std::str::from_utf8(&ben_map! {
                    "failure reason" => ben_bytes!("unable to decode hex string")
                }.encode()).unwrap().to_string()));
        }
    };
}

pub async fn http_service_retrieve_remote_ip(request: HttpRequest, data: Data<Arc<TorrentTracker>>) -> Result<IpAddr, ()>
{
    let origin_ip = match request.peer_addr() {
        None => {
            return Err(());
        }
        Some(ip) => {
            ip.ip()
        }
    };
    match request.headers().get(data.config.http_real_ip.clone()) {
        Some(header) => {
            if header.to_str().is_ok() {
                return if let Ok(ip) = IpAddr::from_str(header.to_str().unwrap()) {
                    Ok(ip)
                } else {
                    Err(())
                }
            } else {
                Err(())
            }
        }
        None => {
            Ok(origin_ip)
        }
    }
}

pub async fn http_validate_ip(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> Result<IpAddr, HttpResponse>
{
    return match http_service_retrieve_remote_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            http_service_stats_log(ip, data.clone()).await;
            Ok(ip)
        }
        Err(_) => {
            Err(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "failure reason" => ben_bytes!("unknown origin ip")
                }.encode()))
        }
    }
}

pub fn http_service_query_hashing(query_map_result: Result<HttpServiceQueryHashingMapOk, CustomError>) -> Result<HttpServiceQueryHashingMapOk, HttpServiceQueryHashingMapErr>
{
    match query_map_result {
        Ok(e) => {
            Ok(e)
        }
        Err(e) => {
            Err(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "failure reason" => ben_bytes!(e.to_string())
                }.encode()))
        }
    }
}

pub async fn http_service_maintenance_mode_check(data: Arc<TorrentTracker>) -> Option<HttpResponse>
{
    if maintenance_mode(data).await {
        Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("maintenance mode enabled, please try again later")
            }.encode()))
    } else {
        None
    }
}

pub async fn http_service_check_key_validation(data: Arc<TorrentTracker>, key: String) -> Option<HttpResponse>
{
    if key.len() != 40 {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("invalid key")
            }.encode()));
    }
    let key_decoded: InfoHash = match http_service_decode_hex_hash(key).await {
        Ok(result) => {
            result
        }
        Err(error) => {
            return Some(error)
        }
    };
    if !data.check_key(key_decoded).await {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("unknown key")
            }.encode()));
    }
    None
}

pub async fn http_service_check_user_key_validation(data: Arc<TorrentTracker>, user_key: String) -> Option<HttpResponse>
{
    if user_key.len() != 40 {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("invalid user key")
            }.encode()));
    }
    let user_key_decoded: UserId = match http_service_decode_hex_user_id(user_key).await {
        Ok(result) => {
            result
        }
        Err(error) => {
            return Some(error)
        }
    };
    if !data.check_user_key(user_key_decoded).await {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!("unknown user key")
            }.encode()));
    }
    None
}

pub fn http_check_host_and_port_used(bind_address: String) {
    if cfg!(target_os = "windows") {
        match std::net::TcpListener::bind(&bind_address) {
            Ok(e) => e,
            Err(_) => {
                panic!("Unable to bind to {} ! Exiting...", &bind_address);
            }
        };
    }
}
