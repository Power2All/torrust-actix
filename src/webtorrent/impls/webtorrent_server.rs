use crate::webtorrent::structs::webtorrent_service_data::WebTorrentServiceData;
use crate::stats::enums::stats_event::StatsEvent;
use crate::webtorrent::enums::wt_message::WtMessage;
use crate::webtorrent::enums::wt_message_type::WtMessageType;
use crate::webtorrent::structs::webtorrent_server::WebTorrentConnection;
use crate::webtorrent::structs::wt_announce::WtAnnounce;
use crate::webtorrent::structs::wt_scrape::WtScrape;
use crate::webtorrent::webtorrent::{
    handle_webtorrent_announce,
    handle_webtorrent_scrape
};
use actix::prelude::*;
use actix_web_actors::ws;
use log::{
    debug,
    error,
    info,
    warn
};
use std::net::IpAddr;

impl Actor for WebTorrentConnection {
    type Context = ws::WebsocketContext<Self>;
    fn started(&mut self, _ctx: &mut Self::Context) {
        self.data.torrent_tracker.update_stats(
            if self.client_ip.is_some_and(|ip| ip.is_ipv4()) {
                StatsEvent::Wt4ConnectionsHandled
            } else {
                StatsEvent::Wt6ConnectionsHandled
            },
            1
        );
        debug!("[WEBTORRENT] WebSocket connection started");
    }

    fn stopped(&mut self, _ctx: &mut Self::Context) {
        debug!("[WEBTORRENT] WebSocket connection stopped");
    }
}

impl StreamHandler<Result<ws::Message, ws::ProtocolError>> for WebTorrentConnection {
    fn handle(&mut self, msg: Result<ws::Message, ws::ProtocolError>, ctx: &mut Self::Context) {
        match msg {
            Ok(ws::Message::Ping(msg)) => {
                ctx.pong(&msg);
            }
            Ok(ws::Message::Pong(_)) => {}
            Ok(ws::Message::Text(text)) => {
                self.handle_text_message(ctx, text.to_string());
            }
            Ok(ws::Message::Binary(data)) => {
                debug!("[WEBTORRENT] Received binary message: {} bytes", data.len());
            }
            Ok(ws::Message::Close(reason)) => {
                debug!("[WEBTORRENT] Close received: {:?}", reason);
                ctx.stop();
            }
            Err(e) => {
                error!("[WEBTORRENT] WebSocket error: {}", e);
                ctx.stop();
            }
            _ => {}
        }
    }
}

impl WebTorrentConnection {
    pub fn new(data: std::sync::Arc<WebTorrentServiceData>, client_ip: Option<std::net::IpAddr>) -> Self {
        Self { data, client_ip }
    }

    pub fn handle_text_message(&mut self, ctx: &mut ws::WebsocketContext<Self>, text: String) {
        debug!("[WEBTORRENT] Received text message: {}", text);
        info!("[WEBTORRENT] Raw JSON (first 500 chars): {}", &text.chars().take(500).collect::<String>());
        let wt_message: WtMessage = match serde_json::from_str(&text) {
            Ok(msg) => msg,
            Err(e) => {
                warn!("[WEBTORRENT] Failed to parse message: {}", e);
                if let Ok(json_val) = serde_json::from_str::<serde_json::Value>(&text) {
                    info!("[WEBTORRENT] Parsed as JSON value: {}", json_val);
                }
                let error_response = serde_json::json!({
                    "action": "error",
                    "error": format!("Invalid message format: {}", e)
                });
                ctx.text(error_response.to_string());
                return;
            }
        };
        info!("[WEBTORRENT] Successfully parsed message: {:?}", wt_message);
        self.handle_wt_message(ctx, wt_message);
    }

    pub fn handle_wt_message(&mut self, ctx: &mut ws::WebsocketContext<Self>, message: WtMessage) {
        let ip = self.client_ip.unwrap_or_else(|| {
            "127.0.0.1".parse().unwrap()
        });
        let tracker = self.data.torrent_tracker.clone();
        match message.message_type() {
            WtMessageType::Announce => {
                tracker.update_stats(
                    if ip.is_ipv4() { StatsEvent::Wt4AnnouncesHandled } else { StatsEvent::Wt6AnnouncesHandled },
                    1
                );
                if let WtMessage::Announce(announce) = message {
                    self.handle_announce(ctx, tracker, announce, ip);
                } else {
                    ctx.text(serde_json::json!({"error": "Message type mismatch"}).to_string());
                }
            }
            WtMessageType::Scrape => {
                tracker.update_stats(
                    if ip.is_ipv4() { StatsEvent::Wt4ScrapesHandled } else { StatsEvent::Wt6ScrapesHandled },
                    1
                );
                if let WtMessage::Scrape(scrape) = message {
                    self.handle_scrape(ctx, tracker, scrape);
                } else {
                    ctx.text(serde_json::json!({"error": "Message type mismatch"}).to_string());
                }
            }
            WtMessageType::Offer => {
                tracker.update_stats(
                    if ip.is_ipv4() { StatsEvent::Wt4OffersHandled } else { StatsEvent::Wt6OffersHandled },
                    1
                );
                // TODO: Implement offer handling for WebRTC signaling
                ctx.text(serde_json::json!({
                    "action": "error",
                    "error": "Offer handling not yet implemented"
                }).to_string());
            }
            WtMessageType::Answer => {
                tracker.update_stats(
                    if ip.is_ipv4() { StatsEvent::Wt4AnswersHandled } else { StatsEvent::Wt6AnswersHandled },
                    1
                );
                // TODO: Implement answer handling for WebRTC signaling
                ctx.text(serde_json::json!({
                    "action": "error",
                    "error": "Answer handling not yet implemented"
                }).to_string());
            }
            WtMessageType::Unknown => {
                ctx.text(serde_json::json!({
                    "action": "error",
                    "error": "Unknown message type"
                }).to_string());
            }
        };
    }

    pub fn handle_announce(&mut self, ctx: &mut ws::WebsocketContext<Self>, tracker: std::sync::Arc<crate::tracker::structs::torrent_tracker::TorrentTracker>, announce: WtAnnounce, ip: IpAddr) {
        let tracker_clone1 = tracker.clone();
        let tracker_clone2 = tracker.clone();
        let announce_clone = announce.clone();
        let ip_clone = ip;
        let fut = async move {
            handle_webtorrent_announce(&tracker_clone1, announce_clone, ip_clone).await
        }.into_actor(self)
        .map(move |result, _actor, ctx| {
            let response = match result {
                Ok(resp) => {
                    let mut json_resp = serde_json::to_value(&resp).unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize response"}));
                    if let Some(obj) = json_resp.as_object_mut() {
                        obj.insert("action".to_string(), serde_json::Value::String("announce".to_string()));
                    }
                    json_resp
                },
                Err(e) => {
                    tracker_clone2.update_stats(
                        if ip_clone.is_ipv4() { StatsEvent::Wt4Failure } else { StatsEvent::Wt6Failure },
                        1
                    );
                    serde_json::json!({
                        "action": "announce",
                        "error": format!("{}", e)
                    })
                }
            };
            let response_str = serde_json::to_string(&response).unwrap_or_default();
            info!("[WEBTORRENT] Sending announce response: {}", response_str);
            ctx.text(response_str);
        });
        ctx.spawn(fut);
    }

    pub fn handle_scrape(&mut self, ctx: &mut ws::WebsocketContext<Self>, tracker: std::sync::Arc<crate::tracker::structs::torrent_tracker::TorrentTracker>, scrape: WtScrape) {
        let tracker_clone1 = tracker.clone();
        let tracker_clone2 = tracker.clone();
        let scrape_clone = scrape.clone();
        let fut = async move {
            handle_webtorrent_scrape(&tracker_clone1, scrape_clone).await
        }.into_actor(self)
        .map(move |result, _actor, ctx| {
            let response = match result {
                Ok(resp) => {
                    serde_json::to_value(resp).unwrap_or_else(|_| serde_json::json!({"error": "Failed to serialize response"}))
                },
                Err(e) => {
                    tracker_clone2.update_stats(
                        StatsEvent::Wt4Failure,
                        1
                    );
                    serde_json::json!({
                        "action": "scrape",
                        "error": format!("{}", e)
                    })
                }
            };
            let response_str = serde_json::to_string(&response).unwrap_or_default();
            debug!("[WEBTORRENT] Sending scrape response: {}", response_str);
            ctx.text(response_str);
        });

        ctx.spawn(fut);
    }
}