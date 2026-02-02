

use std::fs::File;
use std::future::Future;
use std::io::BufReader;
use std::net::SocketAddr;
use std::process::exit;
use std::sync::Arc;
use std::time::Duration;

use actix::prelude::*;
use actix_web::{web, App, Error, HttpRequest, HttpResponse, HttpServer};
use actix_web::dev::ServerHandle;
use actix_web_actors::ws;
use log::{debug, error, info, warn};

use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::config::structs::configuration::Configuration;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::websocket::encoding::encoder::{decode, encode};
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::handshake::{HandshakeRequest, HandshakeResponse, CLUSTER_PROTOCOL_VERSION};

use super::handler::process_cluster_request;

pub struct WebSocketServiceData {
    pub tracker: Arc<TorrentTracker>,
    pub config: Arc<Configuration>,
    
    pub master_id: String,
}

pub struct ClusterConnection {
    
    data: Arc<WebSocketServiceData>,
    
    authenticated: bool,
    
    slave_id: Option<String>,
    
    encoding: ClusterEncoding,
}

impl ClusterConnection {
    pub fn new(data: Arc<WebSocketServiceData>) -> Self {
        Self {
            encoding: data.config.tracker_config.cluster_encoding.clone(),
            data,
            authenticated: false,
            slave_id: None,
        }
    }

    
    fn handle_binary(&mut self, ctx: &mut ws::WebsocketContext<Self>, data: Vec<u8>) {
        if !self.authenticated {
            
            self.handle_handshake(ctx, &data);
            return;
        }

        
        self.handle_cluster_request(ctx, data);
    }

    
    fn handle_handshake(&mut self, ctx: &mut ws::WebsocketContext<Self>, data: &[u8]) {
        
        let handshake: HandshakeRequest = match serde_json::from_slice(data) {
            Ok(req) => req,
            Err(e) => {
                warn!("[WEBSOCKET MASTER] Failed to decode handshake request: {}", e);
                let response = HandshakeResponse::failure("Invalid handshake format".to_string());
                if let Ok(encoded) = serde_json::to_vec(&response) {
                    ctx.binary(encoded);
                }
                ctx.stop();
                return;
            }
        };

        
        if handshake.version != CLUSTER_PROTOCOL_VERSION {
            warn!(
                "[WEBSOCKET MASTER] Protocol version mismatch: expected {}, got {}",
                CLUSTER_PROTOCOL_VERSION, handshake.version
            );
            let response = HandshakeResponse::failure(format!(
                "Protocol version mismatch: expected {}, got {}",
                CLUSTER_PROTOCOL_VERSION, handshake.version
            ));
            if let Ok(encoded) = serde_json::to_vec(&response) {
                ctx.binary(encoded);
            }
            ctx.stop();
            return;
        }

        
        let expected_token = &self.data.config.tracker_config.cluster_token;
        let token_valid = constant_time_compare(&handshake.token, expected_token);

        if !token_valid {
            warn!(
                "[WEBSOCKET MASTER] Authentication failed for slave: {}",
                handshake.slave_id
            );
            
            self.data.tracker.update_stats(StatsEvent::WsAuthFailed, 1);

            let response = HandshakeResponse::failure("Invalid authentication token".to_string());
            if let Ok(encoded) = serde_json::to_vec(&response) {
                ctx.binary(encoded);
            }
            ctx.stop();
            return;
        }

        
        self.authenticated = true;
        self.slave_id = Some(handshake.slave_id.clone());

        info!(
            "[WEBSOCKET MASTER] Slave connected with UUID: {}",
            handshake.slave_id
        );

        
        self.data.tracker.update_stats(StatsEvent::WsAuthSuccess, 1);
        self.data.tracker.update_stats(StatsEvent::WsConnectionsActive, 1);

        
        let response = HandshakeResponse::success(self.encoding.clone(), self.data.master_id.clone());
        if let Ok(encoded) = serde_json::to_vec(&response) {
            ctx.binary(encoded);
        }
    }

    
    fn handle_cluster_request(&mut self, ctx: &mut ws::WebsocketContext<Self>, data: Vec<u8>) {
        
        let request: ClusterRequest = match decode(&self.encoding, &data) {
            Ok(req) => req,
            Err(e) => {
                error!("[WEBSOCKET MASTER] Failed to decode cluster request: {}", e);
                return;
            }
        };

        
        self.data.tracker.update_stats(StatsEvent::WsRequestsReceived, 1);

        let tracker = self.data.tracker.clone();
        let encoding = self.encoding.clone();
        let request_id = request.request_id;

        
        let fut = async move {
            let response = process_cluster_request(tracker, &encoding, request).await;
            (response, encoding)
        };

        
        let fut = fut.into_actor(self).map(move |(response, encoding), act, ctx| {
            
            match encode(&encoding, &response) {
                Ok(encoded) => {
                    ctx.binary(encoded);
                    act.data.tracker.update_stats(StatsEvent::WsResponsesSent, 1);
                }
                Err(e) => {
                    error!("[WEBSOCKET MASTER] Failed to encode response for request {}: {}", request_id, e);
                }
            }
        });

        ctx.spawn(fut);
    }
}

impl Actor for ClusterConnection {
    type Context = ws::WebsocketContext<Self>;

    fn started(&mut self, _ctx: &mut Self::Context) {
        debug!("[WEBSOCKET MASTER] New connection started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        if self.authenticated {
            if let Some(ref slave_id) = self.slave_id {
                info!("[WEBSOCKET MASTER] Slave disconnected with UUID: {}", slave_id);
            }
            self.data.tracker.update_stats(StatsEvent::WsConnectionsActive, -1);
        }
        debug!("[WEBSOCKET MASTER] Connection stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for ClusterConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {
                
            }
            Ok(ws::Message::Binary(data)) => {
                self.handle_binary(ctx, data.to_vec());
            }
            Ok(ws::Message::Text(text)) => {
                
                if !self.authenticated {
                    self.handle_handshake(ctx, text.as_bytes());
                } else {
                    warn!("[WEBSOCKET MASTER] Unexpected text message received");
                }
            }
            Ok(ws::Message::Close(reason)) => {
                debug!("[WEBSOCKET MASTER] Close received: {:?}", reason);
                ctx.stop();
            }
            Err(e) => {
                error!("[WEBSOCKET MASTER] WebSocket error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

async fn websocket_handler(
    req: HttpRequest,
    stream: web::Payload,
    data: web::Data<Arc<WebSocketServiceData>>,
) -> Result<HttpResponse, Error> {
    let connection = ClusterConnection::new(data.get_ref().clone());
    ws::start(connection, &req, stream)
}

pub async fn websocket_master_service(
    addr: SocketAddr,
    tracker: Arc<TorrentTracker>,
) -> (ServerHandle, impl Future<Output = Result<(), std::io::Error>>) {
    let config = tracker.config.clone();
    let keep_alive = config.tracker_config.cluster_keep_alive;
    let request_timeout = config.tracker_config.cluster_request_timeout;
    let disconnect_timeout = config.tracker_config.cluster_disconnect_timeout;
    let worker_threads = config.tracker_config.cluster_threads as usize;
    let max_connections = config.tracker_config.cluster_max_connections as usize;

    
    let master_id = uuid::Uuid::new_v4().to_string();
    info!("[WEBSOCKET MASTER] Master UUID: {}", master_id);

    let service_data = Arc::new(WebSocketServiceData {
        tracker: tracker.clone(),
        config: config.clone(),
        master_id,
    });

    if config.tracker_config.cluster_ssl {
        info!("[WEBSOCKET MASTER] Starting WSS server on {}", addr);

        let ssl_key = &config.tracker_config.cluster_ssl_key;
        let ssl_cert = &config.tracker_config.cluster_ssl_cert;

        if ssl_key.is_empty() || ssl_cert.is_empty() {
            error!("[WEBSOCKET MASTER] No SSL key or SSL certificate given, exiting...");
            exit(1);
        }

        let key_file = &mut BufReader::new(match File::open(ssl_key) {
            Ok(data) => data,
            Err(e) => {
                sentry::capture_error(&e);
                panic!("[WEBSOCKET MASTER] SSL key unreadable: {}", e);
            }
        });

        let certs_file = &mut BufReader::new(match File::open(ssl_cert) {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL cert unreadable: {}", e),
        });

        let tls_certs = match rustls_pemfile::certs(certs_file).collect::<Result<Vec<_>, _>>() {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL cert couldn't be extracted: {}", e),
        };

        let tls_key = match rustls_pemfile::pkcs8_private_keys(key_file).next().unwrap() {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL key couldn't be extracted: {}", e),
        };

        let tls_config = match rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(tls_certs, rustls::pki_types::PrivateKeyDer::Pkcs8(tls_key))
        {
            Ok(data) => data,
            Err(e) => panic!("[WEBSOCKET MASTER] SSL config couldn't be created: {}", e),
        };

        let server = HttpServer::new(move || {
            App::new()
                .app_data(web::Data::new(service_data.clone()))
                .route("/cluster", web::get().to(websocket_handler))
        })
        .keep_alive(Duration::from_secs(keep_alive))
        .client_request_timeout(Duration::from_secs(request_timeout))
        .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
        .workers(worker_threads)
        .max_connections(max_connections)
        .bind_rustls_0_23((addr.ip(), addr.port()), tls_config)
        .unwrap()
        .disable_signals()
        .run();

        return (server.handle(), server);
    }

    info!("[WEBSOCKET MASTER] Starting WS server on {}", addr);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(service_data.clone()))
            .route("/cluster", web::get().to(websocket_handler))
    })
    .keep_alive(Duration::from_secs(keep_alive))
    .client_request_timeout(Duration::from_secs(request_timeout))
    .client_disconnect_timeout(Duration::from_secs(disconnect_timeout))
    .workers(worker_threads)
    .max_connections(max_connections)
    .bind((addr.ip(), addr.port()))
    .unwrap()
    .disable_signals()
    .run();

    (server.handle(), server)
}

fn constant_time_compare(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }

    let mut result = 0u8;
    for (x, y) in a.bytes().zip(b.bytes()) {
        result |= x ^ y;
    }
    result == 0
}
