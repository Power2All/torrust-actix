use std::sync::Arc;
use std::time::Duration;
use actix_web::{rt, web, Error, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_ws::{AggregatedMessage, AggregatedMessageStream, MessageStream, ProtocolError, Session};
use futures_util::stream::Next;
use futures_util::{SinkExt, StreamExt};
use log::{error, info};
use serde_json::{json, Value};
use tokio::time::{timeout, Timeout};
use tokio::time::error::Elapsed;
use uuid::Uuid;
use crate::api::api::{api_service_token, api_validation};
use crate::api::structs::api_service_data::ApiServiceData;
use crate::api::structs::query_token::QueryToken;

#[tracing::instrument(skip(payload), level = "trace")]
pub async fn api_service_cluster_get(request: HttpRequest, payload: web::Payload, data: Data<Arc<ApiServiceData>>) -> HttpResponse
{
    // Validate client
    if let Some(error_return) = api_validation(&request, &data).await { return error_return; }

    // Parse the Params
    let params = web::Query::<QueryToken>::from_query(request.query_string()).unwrap();
    if let Some(response) = api_service_token(params.token.clone(), data.torrent_tracker.config.clone()).await { return response; }

    // Set up the stream to upgrade it to a websocket
    let (resource, mut session, mut payload) = match actix_ws::handle(&request, payload) {
        Ok((resource, session, payload)) => {
            (resource, session, payload)
        }
        Err(error) => {
            return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": error.to_string()}));
        }
    };
    let mut stream = payload.aggregate_continuations().max_continuation_size(2_usize.pow(20));

    // Spawn the websocket thread and handle the traffic further until the connection breaks
    let tracker_ref = data.clone();
    rt::spawn(async move {
        info!("Websocket connection established, requesting server ID");
        
        let mut server_id = String::new();
        
        // Handle initial server ID message
        if let Some(Ok(AggregatedMessage::Text(text))) = stream.next().await {
            let response = match serde_json::from_str::<Value>(&text) {
                Ok(data_response) => { data_response }
                Err(error) => {
                    error!("Failed to receive server ID message: {}", error);
                    return;
                }
            };
            // info!("{:#?}", response);
            if response.get("version").is_some() && response.get("server_id").is_some() {
                let response_server_version = response.get("server_id").unwrap().as_str().unwrap();
                let response_server_id = response.get("server_id").unwrap().as_str().unwrap();
                info!("Received initial server ID: {}", response_server_id);
                match Uuid::parse_str(&response_server_id) {
                    Ok(_) => {
                        server_id = text.clone().to_uppercase();
                        data.torrent_tracker.servers_id.clone().push(response_server_id.clone().parse().unwrap());
                        info!("Websocket connection registered - ID: {}", response_server_id);
                        let transmit = json!({
                            "version": env!("CARGO_PKG_VERSION"),
                            "server_id": tracker_ref.torrent_tracker.server_id.clone()
                        });
                        if let Err(e) = session.text(transmit.to_string()).await {
                            error!("Failed to send server ID message: {}", e);
                            return;
                        }
                    }
                    Err(_) => {
                        error!("Invalid server ID format received");
                        if let Err(e) = session.close(None).await {
                            error!("Failed to close session: {}", e);
                        }
                        return;
                    }
                }
            } else {
                error!("Failed to receive server ID message");
            }
        } else {
            error!("No initial server ID received");
            if let Err(e) = session.close(None).await {
                error!("Failed to close session: {}", e);
            }
            return;
        }

        // Handle subsequent messages
        loop {
            let result = timeout(Duration::from_secs(30), stream.next()).await;
            match result {
                Ok(Some(Ok(AggregatedMessage::Ping(msg)))) => {
                    if let Err(e) = session.pong(&msg).await {
                        info!("Failed to send pong: {}", e);
                        break;
                    }
                }
                Ok(Some(Ok(AggregatedMessage::Close(reason)))) => {
                    info!("Received close message: {:?}", reason);
                    break;
                }
                Ok(Some(Ok(AggregatedMessage::Text(text)))) => {
                    info!("Received text message: {}", text);
                }
                Ok(Some(Ok(AggregatedMessage::Binary(bin))))=> {
                    info!("Received binary message: {:?}", bin);
                }
                Ok(Some(Ok(_))) => {}
                Ok(Some(Err(e))) => {
                    error!("Error: {:?}", e);
                    break;
                }
                Ok(None) => {
                    info!("Stream closed");
                    break;
                }
                Err(_) => {
                    info!("Idle timeout: no message received in 30 seconds");
                    let _ = session.close(None).await;
                    break;
                }
            }
        }

        // Clean up when the connection ends
        info!("Cleaning up...");

        if let Some(id) = data.torrent_tracker.servers_id.clone().iter().position(|x| x == &server_id) {
            info!("Websocket connection removed - ID: {}", id);
            data.torrent_tracker.servers_id.clone().remove(id);
        }
    });

    resource
}