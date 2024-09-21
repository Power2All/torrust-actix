use std::borrow::Cow;
use std::fs::File;
use std::future::Future;
use std::io::{BufReader, Write};
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};
use std::process::exit;
use std::str::FromStr;
use std::sync::Arc;
use std::time::Duration;
use actix_cors::Cors;
use actix_web::{App, http, HttpRequest, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{Data, ServiceConfig};
use bip_bencode::{ben_bytes, ben_int, ben_list, ben_map, BMutAccess};
use log::{debug, error, info};
use crate::common::common::parse_query;
use crate::common::structs::custom_error::CustomError;
use crate::config::structs::api_trackers_config::ApiTrackersConfig;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::http::types::{HttpServiceQueryHashingMapErr, HttpServiceQueryHashingMapOk};
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::enums::torrent_peers_type::TorrentPeersType;
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

pub fn http_service_routes(data: Arc<TorrentTracker>, object: HttpTrackersConfig) -> Box<dyn Fn(&mut ServiceConfig)>
{
    Box::new(move |cfg: &mut ServiceConfig| {
        cfg.app_data(Data::new(data.clone()));
        cfg.app_data(Data::new(object.clone()));
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
    http_server_object: HttpTrackersConfig,
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    let keep_alive = http_server_object.keep_alive.clone().unwrap();
    let request_timeout = http_server_object.request_timeout.clone().unwrap();
    let disconnect_timeout = http_server_object.disconnect_timeout.clone().unwrap();
    let worker_threads = http_server_object.threads.clone().unwrap() as usize;

    if http_server_object.ssl.clone().unwrap() {
        info!("[HTTPS] Starting server listener with SSL on {}", addr);
        if http_server_object.ssl_key.is_none() || http_server_object.ssl_cert.is_none() {
            error!("[HTTPS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }

        let key_file = &mut BufReader::new(File::open(http_server_object.ssl_key.clone().unwrap()).unwrap());
        let certs_file = &mut BufReader::new(File::open(http_server_object.ssl_cert.clone().unwrap()).unwrap());

        let tls_certs = rustls_pemfile::certs(certs_file).collect::<Result<Vec<_>, _>>().unwrap();
        let tls_key = match rustls_pemfile::pkcs8_private_keys(key_file).next().unwrap() {
            Err(_) => { exit(1); }
            Ok(data) => { data }
        };

        let tls_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
            .unwrap();

        let server = HttpServer::new(move || {
            App::new()
                .wrap(http_service_cors())
                .configure(http_service_routes(data.clone(), http_server_object.clone()))
        })
            .keep_alive(Duration::from_secs(keep_alive))
            .client_request_timeout(Duration::from_secs(request_timeout))
            .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
            .workers(worker_threads)
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
            .configure(http_service_routes(data.clone(), http_server_object.clone()))
    })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(request_timeout))
        .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
        .workers(worker_threads)
        .bind((addr.ip(), addr.port()))
        .unwrap()
        .disable_signals()
        .run();

    (server.handle(), server)
}

pub async fn http_service_announce_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone(), object.clone()).await {
        Ok(ip) => {
            http_stat_update(ip, data.clone(), StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => { return result; }
    };

    if data.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
        let key = path.clone();
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(value) = key_check {
            http_stat_update(ip, data.clone(), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return value;
        }
    }

    if data.config.tracker_config.clone().unwrap().users_enabled.unwrap() && !data.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
        let user_key = path.clone();
        let user_key_check = http_service_check_user_key_validation(data.as_ref().clone(), user_key.clone()).await;
        if user_key_check.is_none() {
            return http_service_announce_handler(request, ip, data.as_ref().clone(), Some(http_service_decode_hex_user_id(user_key.clone()).await.unwrap())).await;
        }
    }

    http_service_announce_handler(request, ip, data.as_ref().clone(), None).await
}

pub async fn http_service_announce_userkey(request: HttpRequest, path: web::Path<(String, String)>, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone(), object.clone()).await {
        Ok(ip) => {
            http_stat_update(ip, data.clone(), StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => { return result; }
    };

    if data.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
        let key = path.clone().0;
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(value) = key_check {
            http_stat_update(ip, data.clone(), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return value;
        }
    }

    if data.config.tracker_config.clone().unwrap().users_enabled.unwrap() {
        let user_key = path.clone().1;
        let user_key_check = http_service_check_user_key_validation(data.as_ref().clone(), user_key.clone()).await;
        if user_key_check.is_none() {
            return http_service_announce_handler(request, ip, data.as_ref().clone(), Some(http_service_decode_hex_user_id(user_key.clone()).await.unwrap())).await;
        }
    }

    http_service_announce_handler(request, ip, data.as_ref().clone(), None).await
}

pub async fn http_service_announce(request: HttpRequest, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> HttpResponse
{
    // Validate the IP address
    let ip = match http_validate_ip(request.clone(), data.clone(), object.clone()).await {
        Ok(ip) => {
            http_stat_update(ip, data.clone(), StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => {
            return result;
        }
    };

    if data.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
        http_stat_update(ip, data.clone(), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map!{
            "failure reason" => ben_bytes!("missing key")
        }.encode());
    }

    http_service_announce_handler(request, ip, data.as_ref().clone(), None).await
}

pub async fn http_service_announce_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>, user_key: Option<UserId>) -> HttpResponse
{
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => {
            http_stat_update(ip, Data::new(data.clone()), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
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

    if data.config.tracker_config.clone().unwrap().whitelist_enabled.unwrap() && !data.check_whitelist(announce_unwrapped.info_hash) {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "failure reason" => ben_bytes!("unknown info_hash")
        }.encode());
    }

    if data.config.tracker_config.clone().unwrap().blacklist_enabled.unwrap() && data.check_blacklist(announce_unwrapped.info_hash) {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "failure reason" => ben_bytes!("forbidden info_hash")
        }.encode());
    }

    let (_torrent_peer, torrent_entry) = match data.handle_announce(data.clone(), announce_unwrapped.clone(), user_key).await {
        Ok(result) => { result }
        Err(e) => {
            http_stat_update(ip, Data::new(data.clone()), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode());
        }
    };

    if announce_unwrapped.compact {
        let mut peers_list: Vec<u8> = Vec::new();
        return match ip {
            IpAddr::V4(_) => {
                if announce_unwrapped.left != 0 {
                    let seeds = data.get_peers(
                        torrent_entry.seeds.clone(),
                        TorrentPeersType::IPv4,
                        Some(ip),
                        72
                    );
                    if seeds.is_some() {
                        for (_, torrent_peer) in seeds.unwrap().iter() {
                            let peer_pre_parse = match torrent_peer.peer_addr.ip().to_string().parse::<Ipv4Addr>() {
                                Ok(ip) => { ip }
                                Err(e) => {
                                    error!("[IPV4 Error] {} - {}", torrent_peer.peer_addr.ip().to_string(), e.to_string());
                                    return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                                        "failure reason" => ben_bytes!(e.to_string())
                                    }.encode());
                                }
                            };
                            let _ = peers_list.write(&u32::from(peer_pre_parse).to_be_bytes());
                            peers_list.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
                        }
                    }
                }
                if peers_list.len() != 72 {
                    let peers = data.get_peers(
                        torrent_entry.peers.clone(),
                        TorrentPeersType::IPv4,
                        Some(ip),
                        72
                    );
                    if peers.is_some() {
                        for (_, torrent_peer) in peers.unwrap().iter() {
                            if peers_list.len() != 72 {
                                let peer_pre_parse = match torrent_peer.peer_addr.ip().to_string().parse::<Ipv4Addr>() {
                                    Ok(ip) => { ip }
                                    Err(e) => {
                                        error!("[IPV4 Error] {} - {}", torrent_peer.peer_addr.ip().to_string(), e.to_string());
                                        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                                            "failure reason" => ben_bytes!(e.to_string())
                                        }.encode());
                                    }
                                };
                                let _ = peers_list.write(&u32::from(peer_pre_parse).to_be_bytes());
                                peers_list.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
                                continue;
                            }
                            break;
                        }
                    }
                }
                HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval.unwrap() as i64),
                    "min interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval_minimum.unwrap() as i64),
                    "complete" => ben_int!(torrent_entry.seeds.len() as i64),
                    "incomplete" => ben_int!(torrent_entry.clone().peers.len() as i64),
                    "downloaded" => ben_int!(torrent_entry.completed as i64),
                    "peers" => ben_bytes!(peers_list)
                }.encode())
            }
            IpAddr::V6(_) => {
                if announce_unwrapped.left != 0 {
                    let seeds = data.get_peers(
                        torrent_entry.seeds.clone(),
                        TorrentPeersType::IPv6,
                        Some(ip),
                        72
                    );
                    if seeds.is_some() {
                        for (_, torrent_peer) in seeds.unwrap().iter() {
                            let peer_pre_parse = match torrent_peer.peer_addr.ip().to_string().parse::<Ipv6Addr>() {
                                Ok(ip) => { ip }
                                Err(e) => {
                                    error!("[IPV6 Error] {} - {}", torrent_peer.peer_addr.ip().to_string(), e.to_string());
                                    return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                                            "failure reason" => ben_bytes!(e.to_string())
                                        }.encode());
                                }
                            };
                            let _ = peers_list.write(&u128::from(peer_pre_parse).to_be_bytes());
                            peers_list.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
                        }
                    }
                }
                if peers_list.len() != 72 {
                    let peers = data.get_peers(
                        torrent_entry.peers.clone(),
                        TorrentPeersType::IPv6,
                        Some(ip),
                        72
                    );
                    if peers.is_some() {
                        for (_, torrent_peer) in peers.unwrap().iter() {
                            if peers_list.len() != 72 {
                                let peer_pre_parse = match torrent_peer.peer_addr.ip().to_string().parse::<Ipv6Addr>() {
                                    Ok(ip) => { ip }
                                    Err(e) => {
                                        error!("[IPV6 Error] {} - {}", torrent_peer.peer_addr.ip().to_string(), e.to_string());
                                        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                                            "failure reason" => ben_bytes!(e.to_string())
                                        }.encode());
                                    }
                                };
                                let _ = peers_list.write(&u128::from(peer_pre_parse).to_be_bytes());
                                peers_list.write_all(&announce_unwrapped.clone().port.to_be_bytes()).unwrap();
                                continue;
                            }
                            break;
                        }
                    }
                }
                HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                    "interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval.unwrap() as i64),
                    "min interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval_minimum.unwrap() as i64),
                    "complete" => ben_int!(torrent_entry.seeds.len() as i64),
                    "incomplete" => ben_int!(torrent_entry.peers.len() as i64),
                    "downloaded" => ben_int!(torrent_entry.completed as i64),
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
                    torrent_entry.seeds.clone(),
                    TorrentPeersType::IPv4,
                    Some(ip),
                    72
                );
                if seeds.is_some() {
                    for (peer_id, torrent_peer) in seeds.unwrap().iter() {
                        peers_list_mut.push(ben_map! {
                            "peer id" => ben_bytes!(peer_id.to_string()),
                            "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                            "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                        });
                    }
                }
            }
            if peers_list_mut.len() != 72 {
                let peers = data.get_peers(
                    torrent_entry.peers.clone(),
                    TorrentPeersType::IPv4,
                    Some(ip),
                    72
                );
                if peers.is_some() {
                    for (peer_id, torrent_peer) in peers.unwrap().iter() {
                        if peers_list_mut.len() != 72 {
                            peers_list_mut.push(ben_map! {
                                "peer id" => ben_bytes!(peer_id.to_string()),
                                "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                                "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                            });
                            continue;
                        }
                        break;
                    }
                }
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval_minimum.unwrap() as i64),
                "complete" => ben_int!(torrent_entry.seeds.len() as i64),
                "incomplete" => ben_int!(torrent_entry.peers.len() as i64),
                "downloaded" => ben_int!(torrent_entry.completed as i64),
                "peers" => peers_list
            }.encode())
        }
        IpAddr::V6(_) => {
            if announce_unwrapped.left != 0 {
                let seeds = data.get_peers(
                    torrent_entry.seeds.clone(),
                    TorrentPeersType::IPv6,
                    Some(ip),
                    72
                );
                if seeds.is_some() {
                    for (peer_id, torrent_peer) in seeds.unwrap().iter() {
                        peers_list_mut.push(ben_map! {
                            "peer id" => ben_bytes!(peer_id.to_string()),
                            "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                            "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                        });
                    }
                }
            }
            if peers_list_mut.len() != 72 {
                let peers = data.get_peers(
                    torrent_entry.peers.clone(),
                    TorrentPeersType::IPv6,
                    Some(ip),
                    72
                );
                if peers.is_some() {
                    for (peer_id, torrent_peer) in peers.unwrap().iter() {
                        if peers_list_mut.len() != 72 {
                            peers_list_mut.push(ben_map! {
                                "peer id" => ben_bytes!(peer_id.to_string()),
                                "ip" => ben_bytes!(torrent_peer.peer_addr.ip().to_string()),
                                "port" => ben_int!(torrent_peer.peer_addr.port() as i64)
                            });
                            continue;
                        }
                        break;
                    }
                }
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval_minimum.unwrap() as i64),
                "complete" => ben_int!(torrent_entry.seeds.len() as i64),
                "incomplete" => ben_int!(torrent_entry.peers.len() as i64),
                "downloaded" => ben_int!(torrent_entry.completed as i64),
                "peers6" => peers_list
            }.encode())
        }
    }
}

pub async fn http_service_scrape_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone(), object.clone()).await {
        Ok(ip) => {
            match ip.is_ipv4() {
                true => {
                    data.update_stats(StatsEvent::Tcp4ScrapesHandled, 1);
                }
                false => {
                    data.update_stats(StatsEvent::Tcp6ScrapesHandled, 1);
                }
            }
            ip
        },
        Err(result) => {
            return result;
        }
    };

    debug!("[DEBUG] Request from {}: Scrape with Key", ip);

    if data.config.tracker_config.clone().unwrap().keys_enabled.unwrap() {
        let key = path.into_inner();
        let key_check = http_service_check_key_validation(data.as_ref().clone(), key).await;
        if let Some(value) = key_check { return value; }
    }

    http_service_scrape_handler(request, ip, data.as_ref().clone()).await
}

pub async fn http_service_scrape_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>) -> HttpResponse
{
    let query_map_result = parse_query(Some(request.query_string().to_string()));
    let query_map = match http_service_query_hashing(query_map_result) {
        Ok(result) => { result }
        Err(err) => {
            http_stat_update(ip, Data::new(data.clone()), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return err;
        }
    };

    let scrape = data.validate_scrape(query_map).await;
    if scrape.is_err() {
        http_stat_update(ip, Data::new(data.clone()), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "failure reason" => ben_bytes!(scrape.unwrap_err().to_string())
        }.encode());
    }

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
                "interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval.unwrap() as i64),
                "min interval" => ben_int!(data.config.tracker_config.clone().unwrap().request_interval_minimum.unwrap() as i64),
                "files" => scrape_list
            }.encode())
        }
        Err(e) => {
            http_stat_update(ip, Data::new(data.clone()), StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode())
        }
    }
}

pub async fn http_service_scrape(request: HttpRequest, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone(), object.clone()).await {
        Ok(ip) => {
            match ip.is_ipv4() {
                true => {
                    data.update_stats(StatsEvent::Tcp4ScrapesHandled, 1);
                }
                false => {
                    data.update_stats(StatsEvent::Tcp6ScrapesHandled, 1);
                }
            }
            ip
        },
        Err(result) => {
            return result;
        }
    };

    debug!("[DEBUG] Request from {}: Scrape", ip);

    http_service_scrape_handler(request, ip, data.as_ref().clone()).await
}

pub async fn http_service_not_found(request: HttpRequest, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone(), object.clone()).await {
        Ok(ip) => {
            match ip.is_ipv4() {
                true => { data.update_stats(StatsEvent::Tcp4NotFound, 1); }
                false => { data.update_stats(StatsEvent::Tcp6NotFound, 1); }
            }
            ip
        },
        Err(result) => {
            return result;
        }
    };

    debug!("[DEBUG] Request from {}: 404 Not Found", ip);

    HttpResponse::NotFound().content_type(ContentType::plaintext()).body(std::str::from_utf8(&ben_map! {
        "failure reason" => ben_bytes!("unknown request")
    }.encode()).unwrap().to_string())
}

pub async fn http_service_stats_log(ip: IpAddr, tracker: Data<Arc<TorrentTracker>>)
{
    match ip.is_ipv4() {
        true => { tracker.update_stats(StatsEvent::Tcp4ConnectionsHandled, 1); }
        false => { tracker.update_stats(StatsEvent::Tcp6ConnectionsHandled, 1); }
    }
}

pub async fn http_service_decode_hex_hash(hash: String) -> Result<InfoHash, HttpResponse>
{
    match hex::decode(hash) {
        Ok(hash_result) => { Ok(InfoHash(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap())) }
        Err(_) => {
            Err(HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(std::str::from_utf8(&ben_map! {
                "failure reason" => ben_bytes!("unable to decode hex string")
            }.encode()).unwrap().to_string()))
        }
    }
}

pub async fn http_service_decode_hex_user_id(hash: String) -> Result<UserId, HttpResponse>
{
    match hex::decode(hash) {
        Ok(hash_result) => { Ok(UserId(<[u8; 20]>::try_from(hash_result[0..20].as_ref()).unwrap())) }
        Err(_) => {
            Err(HttpResponse::InternalServerError().content_type(ContentType::plaintext()).body(std::str::from_utf8(&ben_map! {
                "failure reason" => ben_bytes!("unable to decode hex string")
            }.encode()).unwrap().to_string()))
        }
    }
}

pub async fn http_service_retrieve_remote_ip(request: HttpRequest, _data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> Result<IpAddr, ()>
{
    let origin_ip = match request.peer_addr() {
        None => {
            return Err(());
        }
        Some(ip) => {
            ip.ip()
        }
    };
    match request.headers().get(object.real_ip.clone().unwrap()) {
        Some(header) => {
            if header.to_str().is_ok() {
                if let Ok(ip) = IpAddr::from_str(header.to_str().unwrap()) {
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

pub async fn http_validate_ip(request: HttpRequest, data: Data<Arc<TorrentTracker>>, object: Data<&ApiTrackersConfig>) -> Result<IpAddr, HttpResponse>
{
    match http_service_retrieve_remote_ip(request.clone(), data.clone(), object.clone()).await {
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
        Ok(e) => { Ok(e) }
        Err(e) => {
            Err(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode()))
        }
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
        Ok(result) => { result }
        Err(error) => { return Some(error) }
    };
    if !data.check_key(key_decoded) {
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
        Ok(result) => { result }
        Err(error) => { return Some(error) }
    };

    if data.check_user_key(user_key_decoded).is_none() {
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
            Err(_) => { panic!("Unable to bind to {} ! Exiting...", &bind_address); }
        };
    }
}

pub fn http_stat_update(ip: IpAddr, data: Data<Arc<TorrentTracker>>, stats_ipv4: StatsEvent, stat_ipv6: StatsEvent, count: i64)
{
    match ip {
        IpAddr::V4(_) => {
            let data_clone = data.clone();
            data_clone.update_stats(stats_ipv4, count);
        }
        IpAddr::V6(_) => {
            let data_clone = data.clone();
            data_clone.update_stats(stat_ipv6, count);
        }
    }
}