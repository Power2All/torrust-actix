use std::borrow::Cow;
use std::fs::File;
use actix_cors::Cors;
use actix_web::{App, http, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::ServiceConfig;
use scc::ebr::Arc;
use std::future::Future;
use std::io::{BufReader, Write};
use std::net::{IpAddr, SocketAddr};
use std::str::FromStr;
use std::time::Duration;
use actix_extensible_rate_limit::backend::memory::InMemoryBackend;
use actix_extensible_rate_limit::backend::SimpleInputFunctionBuilder;
use actix_extensible_rate_limit::RateLimiter;
use bip_bencode::{ben_map, ben_bytes, ben_list, ben_int, BMutAccess};
use log::info;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use scc::HashIndex;

use crate::common::{CustomError, InfoHash, maintenance_mode, parse_query};
use crate::handlers::{handle_announce, handle_scrape, validate_announce, validate_scrape};
use crate::tracker::TorrentTracker;
use crate::tracker_channels::stats::StatsEvent;

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
        cfg.service(web::resource("announce").route(web::get().to(http_service_announce)));
        cfg.service(web::resource("announce/{key}").route(web::get().to(http_service_announce_key)));
        cfg.service(web::resource("scrape").route(web::get().to(http_service_scrape)));
        cfg.service(web::resource("scrape/{key}").route(web::get().to(http_service_scrape_key)));
        cfg.default_service(web::route().to(http_service_not_found));
    })
}

pub async fn http_service(addr: SocketAddr, data: Arc<TorrentTracker>) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    info!("[SERVICE] Starting server listener on {}", addr);
    let data_cloned = data;
    let server = HttpServer::new(move || {
        let backend = InMemoryBackend::builder().build();
        let input = SimpleInputFunctionBuilder::new(Duration::from_secs(10), 5000).build();
        let middleware = RateLimiter::builder(backend, input).add_headers().build();
        App::new()
            .wrap(http_service_cors())
            .wrap(middleware)
            .configure(http_service_routes(data_cloned.clone()))
    })
        .keep_alive(Duration::from_secs(30))
        .client_request_timeout(Duration::from_secs(5))
        .client_disconnect_timeout(Duration::from_secs(5))
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();
    let handle = server.handle();
    (handle, server)
}

pub async fn https_service(addr: SocketAddr, data: Arc<TorrentTracker>, ssl_key: String, ssl_cert: String) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    info!("[SERVICE] Starting server listener with SSL on {}", addr);
    let data_cloned = data;
    let config = https_service_config(ssl_key, ssl_cert);
    let server = HttpServer::new(move || {
        let backend = InMemoryBackend::builder().build();
        let input = SimpleInputFunctionBuilder::new(Duration::from_secs(10), 5000).build();
        let middleware = RateLimiter::builder(backend, input).add_headers().build();
        App::new()
            .wrap(http_service_cors())
            .wrap(middleware)
            .configure(http_service_routes(data_cloned.clone()))
    })
        .keep_alive(Duration::from_secs(10))
        .client_request_timeout(Duration::from_secs(5))
        .client_disconnect_timeout(Duration::from_secs(5))
        .bind_rustls((addr.ip(), addr.port()), config)
        .unwrap()
        .disable_signals()
        .run();
    let handle = server.handle();
    (handle, server)
}

fn https_service_config(ssl_key: String, ssl_cert: String) -> ServerConfig {
    // init server config builder with safe defaults
    let config = ServerConfig::builder()
        .with_safe_defaults()
        .with_no_client_auth();

    // load TLS key/cert files
    let cert_file = &mut BufReader::new(File::open(ssl_cert).unwrap());
    let key_file = &mut BufReader::new(File::open(ssl_key).unwrap());

    // convert files to key/cert objects
    let cert_chain = certs(cert_file)
        .unwrap()
        .into_iter()
        .map(Certificate)
        .collect();
    let mut keys: Vec<PrivateKey> = pkcs8_private_keys(key_file)
        .unwrap()
        .into_iter()
        .map(PrivateKey)
        .collect();

    // exit if no keys could be parsed
    if keys.is_empty() {
        eprintln!("Could not locate PKCS 8 private keys.");
        std::process::exit(1);
    }

    config.with_single_cert(cert_chain, keys.remove(0)).unwrap()
}

pub async fn http_service_announce_key(request: HttpRequest, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    let ip_check = http_service_retrieve_remote_ip(request.clone()).await;
    if ip_check.is_err() {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown origin ip")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }
    let ip = ip_check.unwrap();
    http_service_stats_log(ip, data.clone()).await;

    if ip.is_ipv4() { data.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1).await; } else { data.update_stats(StatsEvent::Tcp6AnnouncesHandled, 1).await; }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await { return result; }

    // We check if the path is set, and retrieve the possible "key" to check.
    if data.config.keys {
        let key = path.into_inner();
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(key) = key_check { return key; }
    }

    http_service_announce_handler(request, ip, data.as_ref().clone()).await
}

pub async fn http_service_announce(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    let ip_check = http_service_retrieve_remote_ip(request.clone()).await;
    if ip_check.is_err() {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown origin ip")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }
    let ip = ip_check.unwrap();
    http_service_stats_log(ip, data.clone()).await;

    if ip.is_ipv4() { data.update_stats(StatsEvent::Tcp4AnnouncesHandled, 1).await; } else { data.update_stats(StatsEvent::Tcp6AnnouncesHandled, 1).await; }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await { return result; }

    http_service_announce_handler(request, ip, data.as_ref().clone()).await
}

pub async fn http_service_announce_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>) -> HttpResponse
{
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let announce = validate_announce(data.clone().config.clone(), ip, query_map).await;
    let announce_unwrapped = match announce {
        Ok(result) => { result }
        Err(e) => {
            let return_string = (ben_map! {"failure reason" => ben_bytes!(e.to_string())}).encode();
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
        }
    };

    // Check if whitelist is enabled, and if so, check if the torrent hash is known, and if not, show error.
    if data.config.whitelist && !data.check_whitelist(announce_unwrapped.info_hash).await {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown info_hash")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }

    // Check if blacklist is enabled, and if so, check if the torrent hash is known, and if so, show error.
    if data.config.blacklist && data.check_blacklist(announce_unwrapped.info_hash).await {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("forbidden info_hash")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }

    let (_torrent_peer, torrent_entry) = match handle_announce(data.clone(), announce_unwrapped.clone()).await {
        Ok(result) => { result }
        Err(e) => {
            let return_string = (ben_map! {"failure reason" => ben_bytes!(e.to_string())}).encode();
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
        }
    };

    if announce_unwrapped.clone().compact {
        let mut peers: Vec<u8> = Vec::new();
        for (_peer_id, torrent_peer) in torrent_entry.peers.iter() {
            let _ = match torrent_peer.peer_addr.ip() {
                IpAddr::V4(ip) => peers.write(&u32::from(ip).to_be_bytes()),
                IpAddr::V6(ip) => peers.write(&u128::from(ip).to_be_bytes())
            };
            peers.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
        }
        return if announce_unwrapped.clone().remote_addr.is_ipv4() {
            let return_string = (ben_map! {
                "interval" => ben_int!(data.config.interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                "complete" => ben_int!(torrent_entry.seeders),
                "incomplete" => ben_int!(torrent_entry.leechers),
                "downloaded" => ben_int!(torrent_entry.completed),
                "peers" => ben_bytes!(peers.clone())
            }).encode();
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string)
        } else {
            let return_string = (ben_map! {
                "interval" => ben_int!(data.config.interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                "complete" => ben_int!(torrent_entry.seeders),
                "incomplete" => ben_int!(torrent_entry.leechers),
                "downloaded" => ben_int!(torrent_entry.completed),
                "peers6" => ben_bytes!(peers.clone())
            }).encode();
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string)
        };
    }

    let mut peers_list = ben_list!();
    let peers_list_mut = peers_list.list_mut().unwrap();
    for (peer_id, torrent_peer) in torrent_entry.peers.iter() {
        match torrent_peer.peer_addr.ip() {
            IpAddr::V4(_) => {
                peers_list_mut.push(ben_map! {
                    "peer id" => ben_bytes!(peer_id.clone().to_string()),
                    "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                    "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                });
            }
            IpAddr::V6(_) => {
                peers_list_mut.push(ben_map! {
                    "peer id" => ben_bytes!(peer_id.clone().to_string()),
                    "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                    "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                });
            }
        };
    }
    if announce_unwrapped.clone().remote_addr.is_ipv4() {
        let return_string = (ben_map! {
            "interval" => ben_int!(data.config.interval.unwrap() as i64),
            "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
            "complete" => ben_int!(torrent_entry.seeders),
            "incomplete" => ben_int!(torrent_entry.leechers),
            "downloaded" => ben_int!(torrent_entry.completed),
            "peers" => peers_list.clone()
        }).encode();
        HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string)
    } else {
        let return_string = (ben_map! {
            "interval" => ben_int!(data.config.interval.unwrap() as i64),
            "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
            "complete" => ben_int!(torrent_entry.seeders),
            "incomplete" => ben_int!(torrent_entry.leechers),
            "downloaded" => ben_int!(torrent_entry.completed),
            "peers6" => peers_list.clone()
        }).encode();
        HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string)
    }
}

pub async fn http_service_scrape_key(request: HttpRequest, path: web::Path<String>, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    let ip_check = http_service_retrieve_remote_ip(request.clone()).await;
    if ip_check.is_err() {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown origin ip")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }
    let ip = ip_check.unwrap();
    http_service_stats_log(ip, data.clone()).await;

    if ip.is_ipv4() { data.update_stats(StatsEvent::Tcp4ScrapesHandled, 1).await; } else { data.update_stats(StatsEvent::Tcp6ScrapesHandled, 1).await; }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await { return result; }

    // We check if the path is set, and retrieve the possible "key" to check.
    if data.config.keys {
        let key = path.into_inner();
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(key) = key_check { return key; }
    }

    http_service_scrape_handler(request, ip, data.as_ref().clone()).await
}

pub async fn http_service_scrape(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    let ip_check = http_service_retrieve_remote_ip(request.clone()).await;
    if ip_check.is_err() {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown origin ip")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }
    let ip = ip_check.unwrap();
    http_service_stats_log(ip, data.clone()).await;

    if ip.is_ipv4() { data.update_stats(StatsEvent::Tcp4ScrapesHandled, 1).await; } else { data.update_stats(StatsEvent::Tcp6ScrapesHandled, 1).await; }

    if let Some(result) = http_service_maintenance_mode_check(data.as_ref().clone()).await { return result; }

    http_service_scrape_handler(request, ip, data.as_ref().clone()).await
}

pub async fn http_service_scrape_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>) -> HttpResponse
{
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => { return err; }
    };

    let scrape = validate_scrape(data.clone().config.clone(), ip, query_map).await;
    if scrape.is_err() {
        let return_string = (ben_map! {"failure reason" => ben_bytes!(scrape.unwrap_err().to_string())}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }

    match scrape.as_ref() {
        Ok(e) => {
            let data_scrape = handle_scrape(data.clone(), e.clone()).await;
            let mut scrape_list = ben_map!();
            let scrape_list_mut = scrape_list.dict_mut().unwrap();
            for (key, value) in data_scrape.iter() {
                scrape_list_mut.insert(Cow::from(key.0.to_vec()), ben_map! {
                    "complete" => ben_int!(value.seeders),
                    "downloaded" => ben_int!(value.completed),
                    "incomplete" => ben_int!(value.leechers)
                });
            }
            let return_string = (ben_map! {
                "interval" => ben_int!(data.config.interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.interval_minimum.unwrap() as i64),
                "files" => scrape_list
            }).encode();
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string)
        }
        Err(e) => {
            let return_string = (ben_map! {"failure reason" => ben_bytes!(e.to_string())}).encode();
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string)
        }
    }
}

async fn http_service_not_found(request: HttpRequest, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    let ip_check = http_service_retrieve_remote_ip(request.clone()).await;
    if ip_check.is_err() {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown origin ip")}).encode();
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string);
    }
    let ip = ip_check.unwrap();
    http_service_stats_log(ip, data.clone()).await;

    let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown request")}).encode();
    let body = std::str::from_utf8(&return_string).unwrap().to_string();
    HttpResponse::NotFound().content_type(ContentType::plaintext()).body(body)
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
            let return_string = (ben_map! {"failure reason" => ben_bytes!("unable to decode hex string")}).encode();
            let body = std::str::from_utf8(&return_string).unwrap().to_string();
            return Err(HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(body));
        }
    };
}

type HttpServiceQueryHashingMapOk = HashIndex<String, Vec<Vec<u8>>>;
type HttpServiceQueryHashingMapErr = HttpResponse;

pub fn http_service_query_hashing(query_map_result: Result<HttpServiceQueryHashingMapOk, CustomError>) -> Result<HttpServiceQueryHashingMapOk, HttpServiceQueryHashingMapErr>
{
    match query_map_result {
        Ok(e) => {
            Ok(e)
        }
        Err(e) => {
            let return_string = (ben_map! {"failure reason" => ben_bytes!(e.to_string())}).encode();
            Err(HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string))
        }
    }
}

pub async fn http_service_maintenance_mode_check(data: Arc<TorrentTracker>) -> Option<HttpResponse>
{
    if maintenance_mode(data).await {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("maintenance mode enabled, please try again later")}).encode();
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string));
    }
    None
}

pub async fn http_service_check_key_validation(data: Arc<TorrentTracker>, key: String) -> Option<HttpResponse>
{
    if key.len() != 40 {
        let return_string = (ben_map! {"failure reason" => ben_bytes!("invalid key")}).encode();
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string));
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
        let return_string = (ben_map! {"failure reason" => ben_bytes!("unknown key")}).encode();
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(return_string));
    }
    None
}

pub async fn http_service_retrieve_remote_ip(request: HttpRequest) -> Result<IpAddr, ()>
{
    let origin_ip = match request.peer_addr() {
        None => { return Err(()); }
        Some(ip) => { ip.ip() }
    };
    let cloudflare_ip = request.headers().get("CF-Connecting-IP");
    let xreal_ip = request.headers().get("X-Real-IP");

    // Check if IP is from Cloudflare
    if cloudflare_ip.is_some() && cloudflare_ip.unwrap().to_str().is_ok() {
        let check = IpAddr::from_str(cloudflare_ip.unwrap().to_str().unwrap());
        if check.is_ok() { return Ok(check.unwrap()); }
    };

    // Check if IP is from X-Real-IP
    if xreal_ip.is_some() && xreal_ip.unwrap().to_str().is_ok() {
        let check = IpAddr::from_str(xreal_ip.unwrap().to_str().unwrap());
        if check.is_ok() { return Ok(check.unwrap()); }
    };

    Ok(origin_ip)
}