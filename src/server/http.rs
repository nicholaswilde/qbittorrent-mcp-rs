use axum::http::{HeaderMap, StatusCode};
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
use std::{
    convert::Infallible,
    pin::Pin,
    sync::Arc,
    task::{Context, Poll},
    time::Duration,
};
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tower_http::trace::TraceLayer;
use tracing::{debug, info};
use uuid::Uuid;

use crate::server::mcp::{JsonRpcRequest, McpServer};

type Sessions = Arc<DashMap<String, mpsc::Sender<Result<Event, Infallible>>>>;

#[derive(Clone)]
struct AppState {
    mcp_server: McpServer,
    sessions: Sessions,
    auth_token: Option<String>,
}

#[derive(Deserialize)]
struct MessageParams {
    session_id: String,
}

/// SSE stream that removes its session from the map when the client disconnects.
struct SessionStream {
    inner: ReceiverStream<Result<Event, Infallible>>,
    sessions: Sessions,
    id: String,
}

impl Stream for SessionStream {
    type Item = Result<Event, Infallible>;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>> {
        Pin::new(&mut self.inner).poll_next(cx)
    }
}

impl Drop for SessionStream {
    fn drop(&mut self) {
        self.sessions.remove(&self.id);
        info!("SSE session closed: {}", self.id);
    }
}

pub async fn create_router(mcp_server: McpServer, auth_token: Option<String>) -> Router {
    let state = AppState {
        mcp_server,
        sessions: Arc::new(DashMap::new()),
        auth_token,
    };

    // Push queued notifications (finished downloads, tool list changes) to every open SSE session.
    let (server, sessions) = (state.mcp_server.clone(), state.sessions.clone());
    tokio::spawn(async move {
        while server.is_running() {
            tokio::time::sleep(Duration::from_millis(500)).await;
            for n in server.take_notifications() {
                let data = n.to_string();
                for s in sessions.iter() {
                    let _ = s
                        .value()
                        .try_send(Ok(Event::default().event("message").data(data.clone())));
                }
            }
        }
    });

    Router::new()
        .route("/sse", get(sse_handler))
        .route("/message", post(message_handler))
        .route(
            "/mcp",
            post(streamable_handler).get(|| async { StatusCode::METHOD_NOT_ALLOWED }),
        )
        .layer(middleware::from_fn_with_state(
            state.clone(),
            auth_middleware,
        ))
        .layer(TraceLayer::new_for_http())
        .with_state(state)
}

pub async fn run_http_server(
    mcp_server: McpServer,
    host: &str,
    port: u16,
    auth_token: Option<String>,
) -> anyhow::Result<()> {
    let app = create_router(mcp_server, auth_token).await;

    let addr = format!("{}:{}", host, port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .map_err(|e| anyhow::anyhow!("Cannot listen on {}: {}", addr, e))?;
    info!(
        "Starting HTTP MCP Server on {} (SSE: /sse, Streamable HTTP: /mcp)",
        addr
    );
    axum::serve(listener, app).await?;

    Ok(())
}

async fn sse_handler(State(state): State<AppState>) -> impl IntoResponse {
    let session_id = Uuid::new_v4().to_string();
    let (tx, rx) = mpsc::channel(100);

    state.sessions.insert(session_id.clone(), tx.clone());
    info!("New SSE session connected: {}", session_id);

    let endpoint_url = format!("/message?session_id={}", session_id);
    let _ = tx
        .send(Ok(Event::default().event("endpoint").data(endpoint_url)))
        .await;

    Sse::new(SessionStream {
        inner: ReceiverStream::new(rx),
        sessions: state.sessions.clone(),
        id: session_id,
    })
    .keep_alive(axum::response::sse::KeepAlive::new().interval(Duration::from_secs(15)))
}

async fn message_handler(
    State(state): State<AppState>,
    Query(params): Query<MessageParams>,
    Json(req): Json<JsonRpcRequest>,
) -> impl IntoResponse {
    let Some(tx) = state.sessions.get(&params.session_id).map(|s| s.clone()) else {
        return (StatusCode::NOT_FOUND, "Session not found").into_response();
    };

    let mcp = state.mcp_server.clone();
    tokio::spawn(async move {
        debug!(
            "Received HTTP request for session {}: {:?}",
            params.session_id, req
        );
        if let Some(resp) = mcp.handle_rpc(req).await
            && let Ok(data) = serde_json::to_string(&resp)
        {
            let _ = tx
                .send(Ok(Event::default().event("message").data(data)))
                .await;
        }
    });

    (StatusCode::ACCEPTED, "Accepted").into_response()
}

/// Streamable HTTP transport (MCP 2025-03-26): one JSON-RPC message per POST, JSON response.
// shortcut: no server-initiated stream on GET /mcp and no batch requests; clients that need push notifications should use /sse.
async fn streamable_handler(
    State(state): State<AppState>,
    Json(req): Json<JsonRpcRequest>,
) -> Response {
    match state.mcp_server.handle_rpc(req).await {
        Some(resp) => Json(resp).into_response(),
        None => StatusCode::ACCEPTED.into_response(),
    }
}

/// Accepts only `Authorization: Bearer <token>`; a token in the URL would end up in logs and history.
async fn auth_middleware(
    State(state): State<AppState>,
    headers: HeaderMap,
    req: Request,
    next: Next,
) -> Result<Response, StatusCode> {
    if let Some(token) = &state.auth_token {
        let presented = headers
            .get("Authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "));
        if presented != Some(token.as_str()) {
            return Err(StatusCode::UNAUTHORIZED);
        }
    }
    Ok(next.run(req).await)
}
