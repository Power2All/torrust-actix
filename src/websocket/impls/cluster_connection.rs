use crate::stats::enums::stats_event::StatsEvent;
use crate::websocket::structs::cluster_connection::ClusterConnection;
use crate::websocket::structs::cluster_request::ClusterRequest;
use crate::websocket::structs::handshake::{
    HandshakeRequest,
    HandshakeResponse,
    CLUSTER_PROTOCOL_VERSION
};
use crate::websocket::websocket::{
    decode,
    encode,
    process_cluster_request
};
use actix::{
    Actor,
    ActorContext,
    ActorFutureExt,
    AsyncContext,
    StreamHandler,
    WrapFuture
};
use actix_web_actors::ws;
use log::{
    debug,
    error,
    info,
    warn
};

impl ClusterConnection {
    pub fn new(data: std::sync::Arc<crate::websocket::structs::websocket_service_data::WebSocketServiceData>) -> Self {
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

        let token_valid = crate::websocket::websocket::constant_time_compare(&handshake.token, &self.data.config.tracker_config.cluster_token);
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
            Ok(ws::Message::Pong(_)) => {}
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