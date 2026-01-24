#![allow(clippy::collapsible_if)]
use axum::http::StatusCode;
use axum::{
    Json, Router,
    extract::{Query, Request, State},
    middleware::{self, Next},
    response::{
        IntoResponse, Response,
        sse::{Event, Sse},
    },
    routing::{get, post},
};
use dashmap::DashMap;
use futures::stream::Stream;
use serde::Deserialize;
use std::{collections::HashMap, convert::Infallible, sync::Arc, time::Duration};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::{cors::CorsLayer, trace::TraceLayer};
use tracing::{debug, error, info};
use uuid::Uuid;

use crate::server::mcp::{JsonRpcError, JsonRpcRequest, JsonRpcResponse, McpServer};

#[derive(Clone)]
struct AppState {
    mcp_server: McpServer,
    sessions: Arc<DashMap<String, mpsc::Sender<Result<Event, Infallible>>>>,
    auth_token: Option<String>,
}

#[derive(Deserialize)]
struct MessageParams {
    session_id: String,
}

pub async fn run_http_server(
    mcp_server: McpServer,
    host: &str,
    port: u16,
    auth_token: Option<String>,
) -> anyhow::Result<()> {
    let state = AppState {
        mcp_server,
        sessions: Arc::new(DashMap::new()),
        auth_token,
    };

    let app = Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .with_state(state);

    let addr = format!("{}:{}", host, port);
    info!("Starting HTTP MCP Server on {}", addr);

    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(listener, app).await?;

    Ok(())
}

async fn sse_handler(
    State(state): State<AppState>,
) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel(100);

    state.sessions.insert(session_id.clone(), tx.clone());

    info!("New SSE session connected: {}", session_id);

    // Send the endpoint event immediately
    let endpoint_url = format!("/message?session_id={}", session_id);
    let _ = tx
        .send(Ok(Event::default().event("endpoint").data(endpoint_url)))
        .await;

    // Create a stream that removes the session on drop
    let stream = ReceiverStream::new(rx);

    Sse::new(stream)
        .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn message_handler(
    State(state): State<AppState>,
    Query(params): Query<MessageParams>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let session_id = params.session_id;

    let tx = if let Some(sender) = state.sessions.get(&session_id) {
        sender.clone()
    } else {
        return (axum::http::StatusCode::NOT_FOUND, "Session not found").into_response();
    };

    let mcp = state.mcp_server.clone();

    tokio::spawn(async move {
        let req_id = req.id.clone();
        debug!(
            "Received HTTP request for session {}: {:?}",
            session_id, req
        );

        let resp = mcp.handle_request(req).await;

        if let Some(id) = req_id {
            let json_resp = match resp {
                Ok(result) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id),
                    result: Some(result),
                    error: None,
                },
                Err(e) => JsonRpcResponse {
                    jsonrpc: "2.0".to_string(),
                    id: Some(id),
                    result: None,
                    error: Some(JsonRpcError {
                        code: -32603,
                        message: e.to_string(),
                        data: None,
                    }),
                },
            };

            if let Ok(data) = serde_json::to_string(&json_resp) {
                // Send response as 'message' event
                if let Err(e) = tx
                    .send(Ok(Event::default().event("message").data(data)))
                    .await
                {
                    error!("Failed to send SSE event to session {}: {}", session_id, e);
                }
            }

            // Check for notifications
            if mcp.check_notification() {
                let notification = serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "notifications/tools/list_changed"
                });
                if let Ok(data) = serde_json::to_string(&notification) {
                    if let Err(e) = tx
                        .send(Ok(Event::default().event("message").data(data)))
                        .await
                    {
                        error!(
                            "Failed to send notification to session {}: {}",
                            session_id, e
                        );
                    }
                }
            }
        }
    });

    // Return 202 Accepted immediately
    (axum::http::StatusCode::ACCEPTED, "Accepted").into_response()
}

async fn auth_middleware(
    State(state): State<AppState>,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(ref token) = state.auth_token {
        // 1. Check Header
        if let Some(auth_header) = req.headers().get("Authorization") {
            if let Ok(auth_str) = auth_header.to_str() {
                if auth_str == format!("Bearer {}", token) {
                    return Ok(next.run(req).await);
                }
            }
        }

        // 2. Check Query Param
        if let Some(query) = req.uri().query() {
            let params: HashMap<String, String> = url::form_urlencoded::parse(query.as_bytes())
                .into_owned()
                .collect();

            if let Some(t) = params.get("token") {
                if t == token {
                    return Ok(next.run(req).await);
                }
            }
        }

        return Err(StatusCode::UNAUTHORIZED);
    }

    Ok(next.run(req).await)
}
