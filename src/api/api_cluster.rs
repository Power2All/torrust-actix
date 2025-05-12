use std::sync::Arc;
use std::time::Duration;
use actix_web::{rt, web, Error, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_ws::{AggregatedMessage, AggregatedMessageStream, MessageStream, ProtocolError, Session};
use futures_util::stream::Next;
use futures_util::{SinkExt, StreamExt};
use log::info;
use serde_json::json;
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

    // Try to obtain server ID to register, or return an error if the server ID already exists.
    info!("Websocket connection established, requesting server ID");
    match timeout(Duration::from_secs(10), stream.next()).await {
        Ok(ok) => {
            let extracted_id = ok.unwrap();
            if extracted_id.is_ok() {
                match extracted_id.unwrap() {
                    AggregatedMessage::Text(text) => {
                        match Uuid::parse_str(&text) {
                            Ok(_) => {
                                data.torrent_tracker.servers_id.clone().push(text.clone().to_uppercase());
                                info!("Websocket connection registered - ID: {}", text.clone());
                            }
                            Err(_) => {
                                info!("Unable to retrieve server ID, quitting");
                                return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "Invalid server ID"}));   
                            }
                        }
                    }
                    _ => {
                        return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "Invalid server ID"}));
                    }
                }
            }
        }
        Err(_) => {
            info!("Websocket connection took too long, quitting");
            return HttpResponse::BadRequest().content_type(ContentType::json()).json(json!({"status": "Websocket timeout"}));
        }
    }

    // Spawn the websocket thread, and handle the traffic further until the connection breaks
    rt::spawn(async move {
        while let Some(message) = stream.next().await {
            match message {
                Ok(AggregatedMessage::Text(text)) => {
                    info!("Received text message: {}", text);
                }
                Ok(AggregatedMessage::Binary(bin)) => {
                    info!("Received binary message: {:?}", bin);
                }
                Ok(AggregatedMessage::Ping(msg)) => {
                    // Sent a PONG back
                    session.pong(&msg).await.unwrap();
                }
                _ => {}
            }
        }
    });

    resource
}