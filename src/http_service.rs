use std::fs::File;
use actix_cors::Cors;
use actix_remote_ip::RemoteIP;
use actix_web::{App, Error, http, HttpResponse, HttpServer, web};
use actix_web::dev::ServerHandle;
use actix_web::error::{InternalError, JsonPayloadError};
use actix_web::http::header::ContentType;
use actix_web::web::ServiceConfig;
use scc::ebr::Arc;
use serde_json::json;
use std::future::Future;
use std::io::BufReader;
use std::net::{IpAddr, SocketAddr};
use std::time::Duration;
use bip_bencode::{ben_map, ben_bytes};
use log::info;
use rustls::{Certificate, PrivateKey, ServerConfig};
use rustls_pemfile::{certs, pkcs8_private_keys};
use crate::common::InfoHash;
use crate::tracker::{StatsEvent, TorrentTracker};

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
        cfg.app_data(web::JsonConfig::default().error_handler(|err: JsonPayloadError, _| Error::from(InternalError::from_response(err, HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "json parse error"}))))));
        cfg.default_service(web::route().to(http_service_not_found));
    })
}

pub async fn http_service(addr: SocketAddr, data: Arc<TorrentTracker>) -> (ServerHandle, impl Future<Output=Result<(), std::io::Error>>)
{
    info!("[SERVICE] Starting server listener on {}", addr);
    let data_cloned = data;
    let server = HttpServer::new(move || {
        App::new()
            .wrap(http_service_cors())
            .configure(http_service_routes(data_cloned.clone()))
    })
        .keep_alive(Duration::from_secs(10))
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
        App::new()
            .wrap(http_service_cors())
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

async fn http_service_not_found(remote_ip: RemoteIP, data: web::Data<Arc<TorrentTracker>>) -> HttpResponse
{
    http_service_stats_log(remote_ip.0, data.clone()).await;

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
        Err(_) => { return Err(HttpResponse::Ok().content_type(ContentType::json()).json(json!({"status": "unable to decode hex string"}))); }
    };
}