use crate::common::common::parse_query;
use crate::config::enums::cluster_mode::ClusterMode;
use crate::config::structs::http_trackers_config::HttpTrackersConfig;
use crate::http::structs::http_service_data::HttpServiceData;
use crate::security::security::validate_remote_ip;
use crate::ssl::enums::server_identifier::ServerIdentifier;
use crate::ssl::structs::dynamic_certificate_resolver::DynamicCertificateResolver;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::info_hash::InfoHash;
use crate::tracker::structs::announce_entry::AnnounceEntry;
use crate::tracker::structs::announce_query_request::AnnounceQueryRequest;
use crate::tracker::structs::response_peer::ResponsePeer;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::tracker::structs::user_id::UserId;
use crate::websocket::enums::protocol_type::ProtocolType;
use crate::websocket::enums::request_type::RequestType;
use crate::websocket::websocket::{
    create_cluster_error_response,
    forward_request
};
use actix_cors::Cors;
use actix_web::dev::ServerHandle;
use actix_web::http::header::ContentType;
use actix_web::web::{
    Data,
    ServiceConfig
};
use actix_web::{
    middleware::Compress,
    middleware::Condition,
    web,
    App,
    HttpRequest,
    HttpResponse,
    HttpServer
};
use bip_bencode::{
    ben_bytes,
    ben_int,
    ben_list,
    ben_map,
    BMutAccess,
    BencodeMut
};
use log::{
    debug,
    error,
    info
};
use std::borrow::Cow;
use std::future::Future;
use std::net::{
    IpAddr,
    SocketAddr
};
use std::process::exit;
use std::str::FromStr;
use std::sync::{
    Arc,
    LazyLock
};
use std::time::Duration;

static ERR_MISSING_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("missing key") }.encode());
static ERR_UNKNOWN_INFO_HASH: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("unknown info_hash") }.encode());
static ERR_FORBIDDEN_INFO_HASH: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("forbidden info_hash") }.encode());
static ERR_UNKNOWN_REQUEST: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("unknown request") }.encode());
static ERR_UNKNOWN_ORIGIN_IP: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("unknown origin ip") }.encode());
static ERR_INVALID_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("invalid key") }.encode());
static ERR_UNKNOWN_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("unknown key") }.encode());
static ERR_INVALID_USER_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("invalid user key") }.encode());
static ERR_UNKNOWN_USER_KEY: LazyLock<Vec<u8>> = LazyLock::new(|| ben_map!{ "failure reason" => ben_bytes!("unknown user key") }.encode());

/// Builds the permissive CORS policy used by the HTTP tracker endpoints (any origin, GET only).
pub fn http_service_cors() -> Cors
{
    Cors::default()
        .allow_any_origin()
        .send_wildcard()
        .allowed_methods(vec!["GET", "OPTIONS"])
        .allow_any_header()
        .max_age(3600)
}

/// Returns the Actix route configuration for the tracker: `/announce`, `/scrape`, their
/// key/user-key variants, and a catch-all 404 handler.
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
        cfg.service(web::resource("/{key}/{userkey}/announce")
            .route(web::get().to(http_service_announce_userkey))
        );
        // Slash-less variant kept for clients built against earlier releases.
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

/// Starts an HTTP (or HTTPS, when `ssl` is set) tracker listener on `addr`.
///
/// Returns the Actix [`ServerHandle`] for shutdown plus the server future to await.
///
/// # Panics / exit
///
/// Exits the process when the address cannot be bound or the TLS material is missing;
/// panics when the certificate cannot be loaded.
pub async fn http_service(
    addr: SocketAddr,
    data: Arc<TorrentTracker>,
    http_server_object: HttpTrackersConfig,
) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    let tls_config = http_server_object.ssl.then(|| {
        info!("[HTTPS] Starting server listener with SSL on {addr}");
        if http_server_object.ssl_key.is_empty() || http_server_object.ssl_cert.is_empty() {
            error!("[HTTPS] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }
        let server_id = ServerIdentifier::HttpTracker(addr.to_string());
        DynamicCertificateResolver::server_config(&data.certificate_store, server_id, &http_server_object.ssl_cert, &http_server_object.ssl_key)
            .unwrap_or_else(|e| panic!("[HTTPS] Failed to load SSL certificate: {e}"))
    });
    let tag = if tls_config.is_some() { "HTTPS" } else { "HTTP" };
    if tls_config.is_none() {
        info!("[HTTP] Starting server listener on {addr}");
    }
    let service_data = Arc::new(HttpServiceData {
        torrent_tracker: data.clone(),
        http_trackers_config: Arc::new(http_server_object.clone())
    });
    let sentry_enabled = data.config.sentry_config.enabled;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(Compress::default())
            .wrap(Condition::new(sentry_enabled, sentry_actix::Sentry::new()))
            .wrap(http_service_cors())
            .configure(http_service_routes(service_data.clone()))
    })
        .keep_alive(Duration::from_secs(http_server_object.keep_alive))
        .client_request_timeout(Duration::from_secs(http_server_object.request_timeout))
        .client_disconnect_timeout(Duration::from_secs(http_server_object.disconnect_timeout))
        .workers(http_server_object.threads as usize)
        .max_connections(http_server_object.max_connections as usize);
    let server = match tls_config {
        Some(tls_config) => server
            .max_connection_rate(data.config.tracker_config.tls_connection_rate as usize)
            .bind_rustls_0_23((addr.ip(), addr.port()), tls_config),
        None => server.bind((addr.ip(), addr.port())),
    }
        .unwrap_or_else(|e| {
            error!("[{tag}] Unable to bind to {addr}: {e}");
            exit(1);
        })
        .disable_signals()
        .run();
    (server.handle(), server)
}

/// `GET /{key}/announce` — announce with an access key (and, when keys are disabled but
/// users are enabled, a user key) in the path.
pub async fn http_service_announce_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let segment = path.into_inner();
    // One path segment: the access key when keys are on, otherwise the user key.
    let user_key = (!data.torrent_tracker.config.tracker_config.keys_enabled).then(|| segment.clone());
    http_service_announce_with_keys(request, data, segment, user_key).await
}

/// `GET /{key}/{userkey}announce` — announce carrying both an access key and a user key.
pub async fn http_service_announce_userkey(request: HttpRequest, path: web::Path<(String, String)>, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let (key, user_key) = path.into_inner();
    http_service_announce_with_keys(request, data, key, Some(user_key)).await
}

/// The keyed announce routes: checks `key` when keys are enabled and `user_key` when users are,
/// then delegates to [`http_service_announce_handler`].
async fn http_service_announce_with_keys(request: HttpRequest, data: Data<Arc<HttpServiceData>>, key: String, user_key: Option<String>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => { return *result; }
    };
    let tracker_config = &data.torrent_tracker.config.tracker_config;
    if tracker_config.keys_enabled
        && let Some(value) = http_service_check_key_validation(data.torrent_tracker.clone(), key).await {
        data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return value;
    }
    if tracker_config.users_enabled && let Some(user_key) = user_key {
        match http_service_check_user_key_validation(data.torrent_tracker.clone(), user_key).await {
            Ok(user_id) => {
                return http_service_announce_handler(request, ip, data.torrent_tracker.clone(), Some(user_id), data.http_trackers_config.rtctorrent).await;
            }
            Err(response) => {
                data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
                return *response;
            }
        }
    }
    http_service_announce_handler(request, ip, data.torrent_tracker.clone(), None, data.http_trackers_config.rtctorrent).await
}

/// `GET /announce` — plain announce without any key.
///
/// Rejected with `missing key` when the tracker runs in keys-required mode.
pub async fn http_service_announce(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4AnnouncesHandled, StatsEvent::Tcp6AnnouncesHandled, 1);
            ip
        },
        Err(result) => {
            return *result;
        }
    };
    if data.torrent_tracker.config.tracker_config.keys_enabled {
        data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_MISSING_KEY.clone());
    }
    http_service_announce_handler(request, ip, data.torrent_tracker.clone(), None, data.http_trackers_config.rtctorrent).await
}

/// The peer list of a BitTorrent announce response for a client at `client_ip`: up to `numwant`
/// peers of the client's address family, seeds first unless the client is a seed itself, never the
/// client, as compact bytes or BEP 3 dictionaries. Returns the key to file it under (`peers` or
/// `peers6`). Shared by direct announces and ones a cluster master answers for a slave.
pub fn http_announce_peers(entry: &AnnounceEntry, announce: &AnnounceQueryRequest, client_ip: IpAddr) -> (&'static str, BencodeMut<'static>) {
    let (seeds, peers, key) = if client_ip.is_ipv4() {
        (&entry.seeds, &entry.peers, "peers")
    } else {
        (&entry.seeds_ipv6, &entry.peers_ipv6, "peers6")
    };
    let seeds: &[ResponsePeer] = if announce.left != 0 { seeds } else { &[] };
    // Already clamped to 1..=72 by `validate_announce`, so it is safe to size buffers with.
    let want = announce.numwant as usize;
    let chosen = seeds.iter().chain(peers).filter(|peer| peer.peer_id != announce.peer_id).take(want);
    if announce.compact {
        let mut bytes = Vec::with_capacity(want * if client_ip.is_ipv4() { 6 } else { 18 });
        for peer in chosen {
            match peer.peer_addr.ip() {
                IpAddr::V4(ip) => bytes.extend_from_slice(&ip.octets()),
                IpAddr::V6(ip) => bytes.extend_from_slice(&ip.octets()),
            }
            bytes.extend_from_slice(&peer.peer_addr.port().to_be_bytes());
        }
        (key, ben_bytes!(bytes))
    } else {
        let mut list = ben_list!();
        if let Some(entries) = list.list_mut() {
            for peer in chosen {
                entries.push(http_peer_dict(peer, announce.no_peer_id));
            }
        }
        (key, list)
    }
}

/// One entry of a dictionary (non-compact, BEP 3) peer list. `peer id` is the raw 20 bytes BEP 3
/// specifies (it used to go out as 40 hex characters), and is left out when the client sent
/// `no_peer_id`.
pub fn http_peer_dict(peer: &ResponsePeer, no_peer_id: bool) -> BencodeMut<'static> {
    let mut dict = ben_map! {
        "ip" => ben_bytes!(peer.peer_addr.ip().to_string()),
        "port" => ben_int!(i64::from(peer.peer_addr.port()))
    };
    if !no_peer_id && let Some(entries) = dict.dict_mut() {
        entries.insert(std::borrow::Cow::Borrowed(b"peer id"), ben_bytes!(peer.peer_id.0.to_vec()));
    }
    dict
}

/// Core announce logic shared by all announce routes.
///
/// In slave cluster mode the request is forwarded to the master. Otherwise the query is
/// parsed and validated, whitelist/blacklist rules applied, the swarm updated via
/// [`TorrentTracker::handle_announce`], and a bencoded response built: compact peer
/// strings, a dictionary peer list, or an RtcTorrent offer/answer exchange.
pub async fn http_service_announce_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>, user_key: Option<UserId>, rtctorrent_enabled: bool) -> HttpResponse
{
    // Every announce route ends up here. `/announce`, and `/{key}/announce` with keys enabled,
    // arrive without a user key; with users enabled they must be refused, as UDP already does.
    if data.config.tracker_config.users_enabled && user_key.is_none() {
        data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_USER_KEY.clone());
    }
    let query_map = match parse_query(Some(request.query_string())) {
        Ok(result) => result,
        Err(e) => {
            data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode());
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
    let is_rtc_request = announce_unwrapped.rtctorrent.unwrap_or(false);
    // Must precede `handle_announce`, which stores the client's SDP offer/answer, and forwarding.
    if is_rtc_request && !rtctorrent_enabled {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "failure reason" => ben_bytes!(b"rtctorrent not enabled" as &[u8])
        }.encode());
    }
    // A slave forwards only after the checks above: the master never sees the listener's
    // `rtctorrent` setting, and its websocket drops the whole cluster link on an oversized frame,
    // which `parse_query`'s length cap now rules out.
    if data.config.tracker_config.cluster == ClusterMode::slave {
        let query_string = request.query_string().to_string();
        let protocol = if request.connection_info().scheme() == "https" {
            ProtocolType::Https
        } else {
            ProtocolType::Http
        };
        let client_port = request.peer_addr().map_or(0, |a| a.port());
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
                data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
                return HttpResponse::Ok()
                    .content_type(ContentType::plaintext())
                    .body(create_cluster_error_response(&e));
            }
        }
    }
    let tracker_config = &data.config.tracker_config;
    if tracker_config.whitelist_enabled && !data.check_whitelist(announce_unwrapped.info_hash) {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_INFO_HASH.clone());
    }
    if tracker_config.blacklist_enabled && data.check_blacklist(announce_unwrapped.info_hash) {
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_FORBIDDEN_INFO_HASH.clone());
    }
    let torrent_entry = match data.handle_announce(&announce_unwrapped, user_key).await {
        Ok(result) => { result }
        Err(e) => {
            data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode());
        }
    };
    let request_interval = tracker_config.request_interval as i64;
    let request_interval_minimum = tracker_config.request_interval_minimum as i64;
    let rtc_interval = tracker_config.rtc_interval as i64;
    // Counts come from `counts`, never from the peer maps: those are bounded copies.
    let seeds_count = if is_rtc_request {
        torrent_entry.counts.rtc_seeds as i64
    } else {
        torrent_entry.counts.total_seeds() as i64
    };
    let peers_count = if is_rtc_request {
        torrent_entry.counts.rtc_peers as i64
    } else {
        torrent_entry.counts.total_peers() as i64
    };
    let completed_count = torrent_entry.completed as i64;
    if is_rtc_request {
        let mut rtc_peers_list = ben_list!();
        {
            let rtc_peers_list_mut = rtc_peers_list.list_mut().unwrap();
            // Capped by `numwant` (1..=72) like a BitTorrent peer list: up to 128 offers of up to
            // 8 KiB each went out to every leecher, while the client dials at most 8 per round.
            for (peer_id, peer) in &torrent_entry.rtc_seeds {
                if rtc_peers_list_mut.len() >= announce_unwrapped.numwant as usize { break; }
                if *peer_id == announce_unwrapped.peer_id { continue; }
                if let Some(offer) = peer.rtc_sdp_offer()
                    && !offer.is_empty() {
                    rtc_peers_list_mut.push(ben_map! {
                        "peer_id" => ben_bytes!(peer_id.0.to_vec()),
                        "sdp_offer" => ben_bytes!(offer.into_bytes())
                    });
                }
            }
        }
        let pending_answers = data.take_rtc_pending_answers(announce_unwrapped.info_hash, announce_unwrapped.peer_id);
        let mut rtc_answers_list = ben_list!();
        {
            let rtc_answers_list_mut = rtc_answers_list.list_mut().unwrap();
            for (answerer_peer_id, sdp_answer) in &pending_answers {
                rtc_answers_list_mut.push(ben_map! {
                    "peer_id" => ben_bytes!(answerer_peer_id.0.to_vec()),
                    "sdp_answer" => ben_bytes!(sdp_answer.as_bytes().to_vec())
                });
            }
        }

        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "rtc interval" => ben_int!(rtc_interval),
            "complete" => ben_int!(seeds_count),
            "incomplete" => ben_int!(peers_count),
            "rtc_peers" => rtc_peers_list,
            "rtc_answers" => rtc_answers_list
        }.encode());
    }

    let (peers_key, peers) = http_announce_peers(&torrent_entry, &announce_unwrapped, ip);
    HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
        "interval" => ben_int!(request_interval),
        "min interval" => ben_int!(request_interval_minimum),
        "rtc interval" => ben_int!(rtc_interval),
        "complete" => ben_int!(seeds_count),
        "incomplete" => ben_int!(peers_count),
        "downloaded" => ben_int!(completed_count),
        peers_key => peers
    }.encode())
}

/// `GET /{key}/scrape` — scrape with an access key in the path.
///
/// Validates the key before delegating to [`http_service_scrape_handler`].
pub async fn http_service_scrape_key(request: HttpRequest, path: web::Path<String>, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4ScrapesHandled, StatsEvent::Tcp6ScrapesHandled, 1);
            ip
        },
        Err(result) => {
            return *result;
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

/// Core scrape logic shared by the scrape routes.
///
/// In slave cluster mode the request is forwarded to the master; otherwise returns the
/// bencoded `files` dictionary with seed/completed/leech counts per requested info-hash.
pub async fn http_service_scrape_handler(request: HttpRequest, ip: IpAddr, data: Arc<TorrentTracker>) -> HttpResponse
{
    let query_map = match parse_query(Some(request.query_string())) {
        Ok(result) => result,
        Err(e) => {
            data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode());
        }
    };
    // Parsed first so the length cap applies before forwarding too: the master's websocket drops
    // the whole cluster link on an oversized frame.
    if data.config.tracker_config.cluster == ClusterMode::slave {
        let query_string = request.query_string().to_string();
        let protocol = if request.connection_info().scheme() == "https" {
            ProtocolType::Https
        } else {
            ProtocolType::Http
        };
        let client_port = request.peer_addr().map_or(0, |a| a.port());
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
                data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
                return HttpResponse::Ok()
                    .content_type(ContentType::plaintext())
                    .body(create_cluster_error_response(&e));
            }
        }
    }
    let scrape = data.validate_scrape(query_map).await;
    if let Err(scrape) = scrape {
        data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
            "failure reason" => ben_bytes!(scrape.to_string())
        }.encode());
    }
    let tracker_config = &data.config.tracker_config;
    let request_interval = tracker_config.request_interval as i64;
    let request_interval_minimum = tracker_config.request_interval_minimum as i64;
    match scrape.as_ref() {
        Ok(e) => {
            let data_scrape = data.handle_scrape(data.clone(), e).await;
            let mut scrape_list = ben_map!();
            let scrape_list_mut = scrape_list.dict_mut().unwrap();
            for (info_hash, counts) in &data_scrape {
                scrape_list_mut.insert(Cow::from(info_hash.0.to_vec()), ben_map! {
                    "complete" => ben_int!(counts.total_seeds() as i64),
                    "downloaded" => ben_int!(counts.completed as i64),
                    "incomplete" => ben_int!(counts.total_peers() as i64)
                });
            }
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "interval" => ben_int!(request_interval),
                "min interval" => ben_int!(request_interval_minimum),
                "files" => scrape_list
            }.encode())
        }
        Err(e) => {
            data.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
            HttpResponse::Ok().content_type(ContentType::plaintext()).body(ben_map! {
                "failure reason" => ben_bytes!(e.to_string())
            }.encode())
        }
    }
}

/// `GET /scrape` — scrape without an access key.
pub async fn http_service_scrape(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4ScrapesHandled, StatsEvent::Tcp6ScrapesHandled, 1);
            ip
        },
        Err(result) => {
            return *result;
        }
    };
    debug!("[DEBUG] Request from {ip}: Scrape");
    // Same gate as `http_service_announce`: with keys on, an unkeyed scrape would be the one
    // route that still reports swarm sizes for arbitrary info-hashes to anyone who asks.
    // Keyed clients have `/{key}/scrape`.
    if data.torrent_tracker.config.tracker_config.keys_enabled {
        data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4Failure, StatsEvent::Tcp6Failure, 1);
        return HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_MISSING_KEY.clone());
    }
    http_service_scrape_handler(request, ip, data.torrent_tracker.clone()).await
}

/// Catch-all handler returning a bencoded `unknown request` failure with HTTP 404.
pub async fn http_service_not_found(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> HttpResponse
{
    let ip = match http_validate_ip(request.clone(), data.clone()).await {
        Ok(ip) => {
            data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4NotFound, StatsEvent::Tcp6NotFound, 1);
            ip
        },
        Err(result) => {
            return *result;
        }
    };
    debug!("[DEBUG] Request from {ip}: 404 Not Found");
    HttpResponse::NotFound().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_REQUEST.clone())
}

/// Determines the client IP, honouring the configured `real_ip` header when trusted
/// proxies are enabled and the request came from a configured proxy; falls back to the
/// socket peer address.
///
/// # Errors
///
/// Returns `None` when no peer address is available.
pub fn retrieve_remote_ip(request: &HttpRequest, trusted_proxies: bool, trusted_proxy_addrs: &[IpAddr], real_ip: &str) -> Option<IpAddr>
{
    let origin_ip = request.peer_addr().map(|addr| addr.ip())?;
    if !trusted_proxies {
        return Some(origin_ip);
    }
    let explicit_proxies = !trusted_proxy_addrs.is_empty();
    if explicit_proxies && !trusted_proxy_addrs.contains(&origin_ip) {
        return Some(origin_ip);
    }
    request.headers()
        .get(real_ip)
        .and_then(|header| header.to_str().ok())
        .and_then(|ip_str| {
            // Without an allowlist a proxy is indistinguishable from a client, so private
            // address claims are refused; a configured proxy may legitimately forward them.
            validate_remote_ip(ip_str, explicit_proxies).ok()?;
            IpAddr::from_str(ip_str).ok()
        })
        .or(Some(origin_ip))
}

/// Resolves and validates the client IP and logs the connection statistic.
///
/// # Errors
///
/// Returns a bencoded `unknown origin ip` response when the IP cannot be determined.
pub async fn http_validate_ip(request: HttpRequest, data: Data<Arc<HttpServiceData>>) -> Result<IpAddr, Box<HttpResponse>>
{
    let config = &data.http_trackers_config;
    match retrieve_remote_ip(&request, config.trusted_proxies, &config.trusted_proxy_ips, &config.real_ip) {
        Some(ip) => {
            data.torrent_tracker.update_stats_by_family(ip, StatsEvent::Tcp4ConnectionsHandled, StatsEvent::Tcp6ConnectionsHandled, 1);
            Ok(ip)
        }
        None => {
            Err(Box::new(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_ORIGIN_IP.clone())))
        }
    }
}

/// Validates a 40-character hex access key against the key table.
///
/// Returns `None` when the key is valid, or `Some(response)` with the bencoded error to send.
pub async fn http_service_check_key_validation(data: Arc<TorrentTracker>, key: String) -> Option<HttpResponse>
{
    let Ok(key) = key.parse::<InfoHash>() else {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_INVALID_KEY.clone()));
    };
    if !data.check_key(key) {
        return Some(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_KEY.clone()));
    }
    None
}

/// Validates a 40-character hex user key and resolves it to a [`UserId`].
///
/// # Errors
///
/// Returns the bencoded error response when the key is malformed or unknown.
pub async fn http_service_check_user_key_validation(data: Arc<TorrentTracker>, user_key: String) -> Result<UserId, Box<HttpResponse>>
{
    let Ok(user_key) = user_key.parse::<UserId>() else {
        return Err(Box::new(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_INVALID_USER_KEY.clone())));
    };
    if data.check_user_key(user_key).is_none() {
        return Err(Box::new(HttpResponse::Ok().content_type(ContentType::plaintext()).body(ERR_UNKNOWN_USER_KEY.clone())));
    }
    Ok(user_key)
}

/// Windows-only sanity check that a TCP bind address is available before spawning the server.
///
/// # Errors
///
/// Returns the bind error when the address is already in use.
pub fn http_check_host_and_port_used(bind_address: &str) -> std::io::Result<()> {
    if cfg!(target_os = "windows")
        && let Err(e) = std::net::TcpListener::bind(bind_address)
    {
        log::error!("Unable to bind TCP listener to {bind_address}: {e}");
        return Err(e);
    }
    Ok(())
}
