

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use log::{debug, error, info, warn};
use parking_lot::RwLock;
use tokio::sync::oneshot;
use tokio::time::timeout;
use tokio_tungstenite::{connect_async, tungstenite::Message};

use crate::config::enums::cluster_encoding::ClusterEncoding;
use crate::stats::enums::stats_event::StatsEvent;
use crate::tracker::structs::torrent_tracker::TorrentTracker;
use crate::websocket::encoding::encoder::{decode, encode};
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::cluster_response::ClusterResponse;
use crate::websocket::structs::handshake::{HandshakeRequest, HandshakeResponse, CLUSTER_PROTOCOL_VERSION};

type PendingRequestSender = oneshot::Sender<ClusterResponse>;

pub struct SlaveClientState {
    
    pub encoding: Option<ClusterEncoding>,
    
    pub connected: bool,
    
    pub pending_requests: HashMap<u64, PendingRequestSender>,
    
    pub request_counter: u64,
}

impl SlaveClientState {
    pub fn new() -> Self {
        Self {
            encoding: None,
            connected: false,
            pending_requests: HashMap::new(),
            request_counter: 0,
        }
    }

    
    pub fn next_request_id(&mut self) -> u64 {
        self.request_counter = self.request_counter.wrapping_add(1);
        self.request_counter
    }
}

impl Default for SlaveClientState {
    fn default() -> Self {
        Self::new()
    }
}

pub static SLAVE_CLIENT: once_cell::sync::Lazy<Arc<RwLock<SlaveClientState>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(SlaveClientState::new())));

pub static SLAVE_SENDER: once_cell::sync::Lazy<Arc<RwLock<Option<tokio::sync::mpsc::UnboundedSender<Vec<u8>>>>>> =
    once_cell::sync::Lazy::new(|| Arc::new(RwLock::new(None)));

pub fn is_connected() -> bool {
    SLAVE_CLIENT.read().connected
}

pub fn get_encoding() -> Option<ClusterEncoding> {
    SLAVE_CLIENT.read().encoding.clone()
}

pub async fn send_request(
    tracker: &Arc<TorrentTracker>,
    request: ClusterRequest,
) -> Result<ClusterResponse, super::forwarder::ForwardError> {
    
    let (connected, encoding) = {
        let state = SLAVE_CLIENT.read();
        (state.connected, state.encoding.clone())
    };

    if !connected {
        return Err(super::forwarder::ForwardError::NotConnected);
    }

    let encoding = match encoding {
        Some(e) => e,
        None => return Err(super::forwarder::ForwardError::NotConnected),
    };

    
    let encoded = match encode(&encoding, &request) {
        Ok(data) => data,
        Err(e) => return Err(super::forwarder::ForwardError::EncodingError(e.to_string())),
    };

    
    let (tx, rx) = oneshot::channel();
    let request_id = request.request_id;

    
    {
        let mut state = SLAVE_CLIENT.write();
        state.pending_requests.insert(request_id, tx);
    }

    
    let send_result = {
        let sender_guard = SLAVE_SENDER.read();
        if let Some(sender) = sender_guard.as_ref() {
            sender.send(encoded).map_err(|_| ())
        } else {
            Err(())
        }
    };

    match send_result {
        Ok(_) => {
            tracker.update_stats(StatsEvent::WsRequestsSent, 1);
        }
        Err(_) => {
            
            let mut state = SLAVE_CLIENT.write();
            state.pending_requests.remove(&request_id);
            
            let sender_guard = SLAVE_SENDER.read();
            if sender_guard.is_none() {
                return Err(super::forwarder::ForwardError::NotConnected);
            }
            return Err(super::forwarder::ForwardError::ConnectionLost);
        }
    }

    
    let timeout_duration = Duration::from_secs(tracker.config.tracker_config.cluster_request_timeout);

    match timeout(timeout_duration, rx).await {
        Ok(Ok(response)) => {
            tracker.update_stats(StatsEvent::WsResponsesReceived, 1);
            Ok(response)
        }
        Ok(Err(_)) => {
            
            tracker.update_stats(StatsEvent::WsTimeouts, 1);
            Err(super::forwarder::ForwardError::ConnectionLost)
        }
        Err(_) => {
            
            {
                let mut state = SLAVE_CLIENT.write();
                state.pending_requests.remove(&request_id);
            }
            tracker.update_stats(StatsEvent::WsTimeouts, 1);
            Err(super::forwarder::ForwardError::Timeout)
        }
    }
}

pub async fn start_slave_client(tracker: Arc<TorrentTracker>) {
    let config = tracker.config.clone();
    let master_address = &config.tracker_config.cluster_master_address;
    let token = &config.tracker_config.cluster_token;
    let reconnect_interval = config.tracker_config.cluster_reconnect_interval;

    
    let slave_id = hostname::get()
        .map(|h| h.to_string_lossy().to_string())
        .unwrap_or_else(|_| format!("slave-{}", std::process::id()));

    info!("[WEBSOCKET SLAVE] Starting slave client, connecting to {}", master_address);
    info!("[WEBSOCKET SLAVE] Slave ID: {}", slave_id);

    loop {
        match connect_to_master(
            &tracker,
            master_address,
            token,
            &slave_id,
        ).await {
            Ok(()) => {
                info!("[WEBSOCKET SLAVE] Disconnected from master");
            }
            Err(e) => {
                error!("[WEBSOCKET SLAVE] Connection error: {}", e);
            }
        }

        
        {
            let mut state = SLAVE_CLIENT.write();
            state.connected = false;
            state.encoding = None;

            
            for (_, sender) in state.pending_requests.drain() {
                let _ = sender.send(ClusterResponse::error(0, "Connection lost".to_string()));
            }
        }

        
        {
            let mut sender_guard = SLAVE_SENDER.write();
            *sender_guard = None;
        }

        tracker.update_stats(StatsEvent::WsConnectionsActive, -1);
        tracker.update_stats(StatsEvent::WsReconnects, 1);

        info!(
            "[WEBSOCKET SLAVE] Reconnecting in {} seconds...",
            reconnect_interval
        );
        tokio::time::sleep(Duration::from_secs(reconnect_interval)).await;
    }
}

async fn connect_to_master(
    tracker: &Arc<TorrentTracker>,
    master_address: &str,
    token: &str,
    slave_id: &str,
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    info!("[WEBSOCKET SLAVE] Connecting to master: {}", master_address);

    
    let (ws_stream, _) = connect_async(master_address).await?;
    let (mut write, mut read) = ws_stream.split();

    info!("[WEBSOCKET SLAVE] Connected, sending handshake...");

    
    let handshake = HandshakeRequest::new(token.to_string(), slave_id.to_string());
    let handshake_data = serde_json::to_vec(&handshake)?;
    write.send(Message::Binary(handshake_data.into())).await?;

    
    let handshake_response: HandshakeResponse = match read.next().await {
        Some(Ok(Message::Binary(data))) => serde_json::from_slice(&data)?,
        Some(Ok(Message::Text(text))) => serde_json::from_str(&text)?,
        Some(Err(e)) => return Err(format!("WebSocket error during handshake: {}", e).into()),
        None => return Err("Connection closed during handshake".into()),
        _ => return Err("Unexpected message type during handshake".into()),
    };

    if !handshake_response.success {
        let error_msg = handshake_response.error.unwrap_or_else(|| "Unknown error".to_string());
        error!("[WEBSOCKET SLAVE] Handshake failed: {}", error_msg);
        tracker.update_stats(StatsEvent::WsAuthFailed, 1);
        return Err(format!("Handshake failed: {}", error_msg).into());
    }

    
    if handshake_response.version != CLUSTER_PROTOCOL_VERSION {
        warn!(
            "[WEBSOCKET SLAVE] Protocol version mismatch: master={}, slave={}",
            handshake_response.version, CLUSTER_PROTOCOL_VERSION
        );
    }

    let encoding = handshake_response.encoding.unwrap_or(ClusterEncoding::binary);
    info!(
        "[WEBSOCKET SLAVE] Handshake successful, using encoding: {:?}",
        encoding
    );

    tracker.update_stats(StatsEvent::WsAuthSuccess, 1);
    tracker.update_stats(StatsEvent::WsConnectionsActive, 1);

    
    {
        let mut state = SLAVE_CLIENT.write();
        state.connected = true;
        state.encoding = Some(encoding.clone());
    }

    
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<Vec<u8>>();

    
    {
        let mut sender_guard = SLAVE_SENDER.write();
        *sender_guard = Some(tx);
    }

    
    let write_handle = tokio::spawn(async move {
        while let Some(data) = rx.recv().await {
            if write.send(Message::Binary(data.into())).await.is_err() {
                break;
            }
        }
    });

    
    let encoding_for_read = encoding.clone();
    while let Some(msg) = read.next().await {
        match msg {
            Ok(Message::Binary(data)) => {
                handle_response(&encoding_for_read, &data);
            }
            Ok(Message::Ping(data)) => {
                debug!("[WEBSOCKET SLAVE] Received ping");
                
                let _ = data;
            }
            Ok(Message::Pong(_)) => {
                debug!("[WEBSOCKET SLAVE] Received pong");
            }
            Ok(Message::Close(_)) => {
                info!("[WEBSOCKET SLAVE] Received close from master");
                break;
            }
            Err(e) => {
                error!("[WEBSOCKET SLAVE] WebSocket error: {}", e);
                break;
            }
            _ => {}
        }
    }

    
    write_handle.abort();

    Ok(())
}

fn handle_response(encoding: &ClusterEncoding, data: &[u8]) {
    let response: ClusterResponse = match decode(encoding, data) {
        Ok(r) => r,
        Err(e) => {
            error!("[WEBSOCKET SLAVE] Failed to decode response: {}", e);
            return;
        }
    };

    
    let mut state = SLAVE_CLIENT.write();
    if let Some(sender) = state.pending_requests.remove(&response.request_id) {
        let _ = sender.send(response);
    } else {
        warn!(
            "[WEBSOCKET SLAVE] Received response for unknown request: {}",
            response.request_id
        );
    }
}
