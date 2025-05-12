use std::sync::Arc;
use actix_web::{rt, web, Error, HttpRequest, HttpResponse};
use actix_web::http::header::ContentType;
use actix_web::web::Data;
use actix_ws::{AggregatedMessage, MessageStream, Session};
use futures_util::StreamExt;
use log::info;
use serde_json::json;
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

    // Spawn the websocket thread, and handle the traffic further until the connection breaks
    info!("Websocket connection established");
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