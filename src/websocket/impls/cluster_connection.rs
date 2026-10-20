use crate::stats::enums::stats_event::StatsEvent;
use crate::websocket::structs::cluster_connection::ClusterConnection;
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::handshake_request::{
    HandshakeRequest,
    CLUSTER_PROTOCOL_VERSION,
};
use crate::websocket::structs::handshake_response::HandshakeResponse;
use crate::websocket::websocket::{
    decode,
    encode,
    process_cluster_request
};
use actix_ws::{
    AggregatedMessage,
    AggregatedMessageStream,
    Session
};
use log::{
    debug,
    error,
    info,
    warn
};

/// How long a freshly accepted cluster socket may stay unauthenticated before it is dropped.
///
/// The HTTP-level `client_request_timeout` stops applying once the connection is upgraded, so
/// without this a socket that never sends a handshake lives forever and keeps one of the
/// `cluster_max_connections` slots away from a real slave.
const HANDSHAKE_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(30);

impl ClusterConnection {
    /// Creates a master-side connection actor for a newly accepted slave WebSocket.
    pub fn new(data: std::sync::Arc<crate::websocket::structs::websocket_service_data::WebSocketServiceData>) -> Self {
        Self {
            encoding: data.config.tracker_config.cluster_encoding.clone(),
            data,
            slave_id: None,
        }
    }

    /// Serves one slave's socket until it closes: the handshake first, then cluster requests,
    /// each answered from its own task so a slow one does not hold up the rest.
    pub async fn run(mut self, mut session: Session, mut messages: AggregatedMessageStream) {
        debug!("[WEBSOCKET MASTER] New connection started");
        let handshake_deadline = tokio::time::sleep(HANDSHAKE_TIMEOUT);
        tokio::pin!(handshake_deadline);
        loop {
            let message = tokio::select! {
                message = messages.recv() => message,
                () = &mut handshake_deadline, if self.slave_id.is_none() => {
                    warn!("[WEBSOCKET MASTER] Dropping connection that never completed the handshake");
                    break;
                }
            };
            match message {
                Some(Ok(AggregatedMessage::Ping(bytes))) => {
                    if session.pong(&bytes).await.is_err() {
                        break;
                    }
                }
                Some(Ok(AggregatedMessage::Binary(data))) => {
                    if self.slave_id.is_none() {
                        if !self.handle_handshake(&mut session, &data).await {
                            break;
                        }
                    } else {
                        self.handle_cluster_request(&session, &data);
                    }
                }
                Some(Ok(AggregatedMessage::Text(text))) => {
                    if self.slave_id.is_some() {
                        warn!("[WEBSOCKET MASTER] Unexpected text message received");
                    } else if !self.handle_handshake(&mut session, text.as_bytes()).await {
                        break;
                    }
                }
                Some(Ok(AggregatedMessage::Close(reason))) => {
                    debug!("[WEBSOCKET MASTER] Close received: {reason:?}");
                    break;
                }
                Some(Ok(AggregatedMessage::Pong(_))) => {}
                Some(Err(e)) => {
                    error!("[WEBSOCKET MASTER] WebSocket error: {e}");
                    break;
                }
                None => break,
            }
        }
        if let Some(ref slave_id) = self.slave_id {
            info!("[WEBSOCKET MASTER] Slave disconnected with UUID: {slave_id}");
            self.data.tracker.update_stats(StatsEvent::WsConnectionsActive, -1);
        }
        let _ = session.close(None).await;
        debug!("[WEBSOCKET MASTER] Connection stopped");
    }

    /// Checks a slave's handshake and answers it; `false` means the connection must be closed.
    async fn handle_handshake(&mut self, session: &mut Session, data: &[u8]) -> bool {
        let handshake: HandshakeRequest = match serde_json::from_slice(data) {
            Ok(req) => req,
            Err(e) => {
                warn!("[WEBSOCKET MASTER] Failed to decode handshake request: {e}");
                Self::send(session, &HandshakeResponse::failure("Invalid handshake format".to_string())).await;
                return false;
            }
        };

        if handshake.version != CLUSTER_PROTOCOL_VERSION {
            warn!(
                "[WEBSOCKET MASTER] Protocol version mismatch: expected {}, got {}",
                CLUSTER_PROTOCOL_VERSION, handshake.version
            );
            Self::send(session, &HandshakeResponse::failure(format!(
                "Protocol version mismatch: expected {}, got {}",
                CLUSTER_PROTOCOL_VERSION, handshake.version
            ))).await;
            return false;
        }

        let token_valid = crate::security::security::constant_time_eq(&handshake.token, &self.data.config.tracker_config.cluster_token);
        if !token_valid {
            warn!(
                "[WEBSOCKET MASTER] Authentication failed for slave: {}",
                handshake.slave_id
            );
            self.data.tracker.update_stats(StatsEvent::WsAuthFailed, 1);
            Self::send(session, &HandshakeResponse::failure("Invalid authentication token".to_string())).await;
            return false;
        }
        self.slave_id = Some(handshake.slave_id.clone());
        info!(
            "[WEBSOCKET MASTER] Slave connected with UUID: {}",
            handshake.slave_id
        );
        self.data.tracker.update_stats(StatsEvent::WsAuthSuccess, 1);
        self.data.tracker.update_stats(StatsEvent::WsConnectionsActive, 1);
        Self::send(session, &HandshakeResponse::success(self.encoding.clone(), self.data.master_id.clone())).await;
        true
    }

    async fn send(session: &mut Session, response: &HandshakeResponse) {
        if let Ok(encoded) = serde_json::to_vec(response) {
            let _ = session.binary(encoded).await;
        }
    }

    fn handle_cluster_request(&self, session: &Session, data: &[u8]) {
        let request: ClusterRequest = match decode(&self.encoding, data) {
            Ok(req) => req,
            Err(e) => {
                error!("[WEBSOCKET MASTER] Failed to decode cluster request: {e}");
                return;
            }
        };
        self.data.tracker.update_stats(StatsEvent::WsRequestsReceived, 1);
        let tracker = self.data.tracker.clone();
        let encoding = self.encoding.clone();
        let mut session = session.clone();
        let request_id = request.request_id;
        actix_web::rt::spawn(async move {
            let response = process_cluster_request(tracker.clone(), request).await;
            match encode(&encoding, &response) {
                Ok(encoded) => {
                    if session.binary(encoded).await.is_ok() {
                        tracker.update_stats(StatsEvent::WsResponsesSent, 1);
                    }
                }
                Err(e) => {
                    error!("[WEBSOCKET MASTER] Failed to encode response for request {request_id}: {e}");
                }
            }
        });
    }
}
