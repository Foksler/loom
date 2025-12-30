// Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights
// reserved. SPDX-License-Identifier: Proprietary

//! WebSocket communication module for Phase 3.
//!
//! # Overview
//! This module provides WebSocket support for low-latency bi-directional
//! communication between server and client, with integrated authentication.
//!
//! # Phase 3: WebSocket Architecture
//! Current system uses SSE + HTTP polling for query transport. Phase 3 upgrades
//! to a single WebSocket connection that handles all communication types.
//!
//! ## Authentication
//!
//! WebSocket connections support two authentication methods:
//!
//! 1. **Cookie-based auth (Web browsers)**: Session cookie is validated during
//!    the WebSocket upgrade handshake. If valid, connection is immediately authenticated.
//!
//! 2. **First-message auth (CLI/VS Code)**: Client connects without auth, then sends
//!    an auth message within 30 seconds: `{"type": "auth", "token": "lt_xxx"}`.
//!    Connection is closed if auth isn't received in time.
//!
//! ## Benefits
//!
//! 1. **Lower Latency**: No HTTP round-trip overhead per query (~50ms vs current 100-500ms)
//! 2. **Bidirectional**: Server can push to client without polling
//! 3. **Persistent**: Eliminates connection overhead for each message
//! 4. **Backpressure**: Built-in flow control via WebSocket frame buffering
//! 5. **Unified Protocol**: Single connection replaces SSE + HTTP query endpoints

use axum::http;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;
use tracing::{info, instrument, warn};

use loom_auth::{
    auth_timeout, close_code_for_error, extract_bearer_token, extract_session_cookie,
    identify_bearer_token, BearerTokenType, CurrentUser, WsAuthContext, WsAuthError,
    WsAuthMessage, WsAuthMethod, WsAuthResponse, WsAuthState,
};

/// WebSocket connection state for a single client session.
///
/// This struct manages:
/// - Authentication state and timeout
/// - Message routing (queries, responses, LLM events)
/// - Connection lifecycle
/// - Backpressure handling
/// - Per-session state
#[derive(Clone)]
pub struct WebSocketConnection {
    /// Session ID associated with this connection (for debugging/tracing).
    session_id: String,

    /// Connection lifecycle state.
    state: Arc<RwLock<ConnectionState>>,

    /// Authentication state.
    auth_state: Arc<RwLock<WsAuthState>>,

    /// When the connection was established.
    connected_at: Instant,
}

/// Connection lifecycle states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Connection just established, may be awaiting auth.
    Handshaking,

    /// Connection ready for messaging.
    Active,

    /// Connection closing (draining messages).
    Closing,

    /// Connection closed.
    Closed,
}

impl WebSocketConnection {
    /// Create a new unauthenticated WebSocket connection.
    ///
    /// Use this when the client connects without cookies (CLI/VS Code).
    /// The connection will wait up to 30 seconds for an auth message.
    pub fn new_unauthenticated(session_id: String) -> Self {
        Self {
            session_id,
            state: Arc::new(RwLock::new(ConnectionState::Handshaking)),
            auth_state: Arc::new(RwLock::new(WsAuthState::AwaitingAuth)),
            connected_at: Instant::now(),
        }
    }

    /// Create a new pre-authenticated WebSocket connection.
    ///
    /// Use this when the client authenticated via session cookie during upgrade.
    pub fn new_authenticated(session_id: String, current_user: CurrentUser) -> Self {
        let auth_ctx = WsAuthContext::new(current_user, WsAuthMethod::SessionCookie);
        Self {
            session_id,
            state: Arc::new(RwLock::new(ConnectionState::Active)),
            auth_state: Arc::new(RwLock::new(WsAuthState::Authenticated(Box::new(auth_ctx)))),
            connected_at: Instant::now(),
        }
    }

    /// Get the session ID for this connection.
    pub fn session_id(&self) -> &str {
        &self.session_id
    }

    /// Get current connection state.
    #[instrument(skip(self))]
    pub async fn connection_state(&self) -> ConnectionState {
        *self.state.read().await
    }

    /// Get current authentication state.
    pub async fn auth_state(&self) -> WsAuthState {
        self.auth_state.read().await.clone()
    }

    /// Check if the connection is authenticated.
    pub async fn is_authenticated(&self) -> bool {
        self.auth_state.read().await.is_authenticated()
    }

    /// Check if the authentication timeout has elapsed.
    ///
    /// Returns true if:
    /// - Connection is awaiting auth AND
    /// - More than 30 seconds have passed since connection
    pub fn is_auth_timeout_elapsed(&self) -> bool {
        self.connected_at.elapsed() > auth_timeout()
    }

    /// Get remaining time for authentication.
    ///
    /// Returns None if already authenticated or timeout has elapsed.
    pub async fn auth_time_remaining(&self) -> Option<std::time::Duration> {
        if self.is_authenticated().await {
            return None;
        }
        let elapsed = self.connected_at.elapsed();
        let timeout = auth_timeout();
        if elapsed >= timeout {
            None
        } else {
            Some(timeout - elapsed)
        }
    }

    /// Process an authentication message from the client.
    ///
    /// This is called when a CLI/VS Code client sends the first message
    /// after connecting. The message should be: `{"type": "auth", "token": "lt_xxx"}`
    ///
    /// # Arguments
    ///
    /// * `message` - The raw JSON message from the client
    /// * `validate_token` - Async function to validate the token and return CurrentUser
    ///
    /// # Returns
    ///
    /// * `Ok(WsAuthResponse)` - Authentication result to send back to client
    /// * `Err(WsAuthError)` - If parsing or validation fails
    #[instrument(skip(self, validate_token))]
    pub async fn process_auth_message<F, Fut>(
        &self,
        message: &str,
        validate_token: F,
    ) -> Result<WsAuthResponse, WsAuthError>
    where
        F: FnOnce(String, BearerTokenType) -> Fut,
        Fut: std::future::Future<Output = Result<CurrentUser, WsAuthError>>,
    {
        // Check if already authenticated
        if self.is_authenticated().await {
            warn!(session_id = %self.session_id, "Already authenticated, ignoring auth message");
            let auth_state = self.auth_state.read().await;
            if let Some(ctx) = auth_state.user() {
                return Ok(WsAuthResponse::success(ctx.user_id()));
            }
        }

        // Check for timeout
        if self.is_auth_timeout_elapsed() {
            let error = WsAuthError::Timeout;
            *self.auth_state.write().await = WsAuthState::Failed(error.clone());
            return Err(error);
        }

        // Parse the auth message
        let auth_msg: WsAuthMessage = serde_json::from_str(message).map_err(|e| {
            warn!(session_id = %self.session_id, error = %e, "Failed to parse auth message");
            WsAuthError::InvalidAuthMessage
        })?;

        // Validate message format
        if !auth_msg.is_valid() {
            warn!(session_id = %self.session_id, "Invalid auth message format");
            let error = WsAuthError::InvalidAuthMessage;
            *self.auth_state.write().await = WsAuthState::Failed(error.clone());
            return Err(error);
        }

        // Identify token type
        let token_type = identify_bearer_token(&auth_msg.token);
        let auth_method = match token_type {
            BearerTokenType::AccessToken => WsAuthMethod::BearerToken,
            BearerTokenType::ApiKey => WsAuthMethod::ApiKey,
            BearerTokenType::Unknown => {
                warn!(session_id = %self.session_id, "Unknown token type");
                let error = WsAuthError::InvalidToken;
                *self.auth_state.write().await = WsAuthState::Failed(error.clone());
                return Err(error);
            }
        };

        // Validate the token
        let current_user = validate_token(auth_msg.token, token_type).await.map_err(|e| {
            warn!(session_id = %self.session_id, error = ?e, "Token validation failed");
            *futures::executor::block_on(self.auth_state.write()) = WsAuthState::Failed(e.clone());
            e
        })?;

        // Authentication successful
        let user_id = current_user.user.id;
        let auth_ctx = WsAuthContext::new(current_user, auth_method);

        info!(
            session_id = %self.session_id,
            user_id = %user_id,
            auth_method = ?auth_method,
            "WebSocket authenticated successfully"
        );

        *self.auth_state.write().await = WsAuthState::Authenticated(Box::new(auth_ctx));
        *self.state.write().await = ConnectionState::Active;

        Ok(WsAuthResponse::success(&user_id))
    }

    /// Mark the connection as closing.
    pub async fn start_closing(&self) {
        *self.state.write().await = ConnectionState::Closing;
    }

    /// Mark the connection as closed.
    pub async fn close(&self) {
        *self.state.write().await = ConnectionState::Closed;
    }

    /// Handle authentication timeout.
    ///
    /// Call this when the auth timeout fires. Sets the auth state to failed
    /// and returns the appropriate close code.
    pub async fn handle_auth_timeout(&self) -> u16 {
        let error = WsAuthError::Timeout;
        *self.auth_state.write().await = WsAuthState::Failed(error.clone());
        *self.state.write().await = ConnectionState::Closing;
        close_code_for_error(&error)
    }
}

/// Extract authentication from WebSocket upgrade request headers.
///
/// This function checks for session cookie authentication during the
/// WebSocket upgrade handshake. Used for web browser connections.
///
/// # Arguments
///
/// * `headers` - HTTP headers from the upgrade request
/// * `validate_session` - Async function to validate session and return CurrentUser
///
/// # Returns
///
/// * `Some(CurrentUser)` if session cookie is valid
/// * `None` if no cookie or invalid session
#[instrument(skip(headers, validate_session))]
pub async fn extract_cookie_auth<F, Fut>(
    headers: &http::HeaderMap,
    validate_session: F,
) -> Option<CurrentUser>
where
    F: FnOnce(String) -> Fut,
    Fut: std::future::Future<Output = Option<CurrentUser>>,
{
    let session_token = extract_session_cookie(headers)?;
    validate_session(session_token).await
}

/// Extract bearer token from upgrade request for immediate auth.
///
/// Some clients may pass the bearer token in the initial upgrade request
/// via the Authorization header or query parameter.
pub fn extract_upgrade_bearer_token(headers: &http::HeaderMap) -> Option<String> {
    extract_bearer_token(headers)
}

/// Message types for WebSocket communication.
///
/// # Mapping from HTTP/SSE
/// - ServerQuery: HTTP POST → WebSocket message
/// - ServerQueryResponse: HTTP POST → WebSocket message
/// - LlmStreamEvent: SSE event → WebSocket message
/// - LlmCompletion: SSE final event → WebSocket message
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum WebSocketMessage {
    /// Authentication message from client.
    Auth(WsAuthMessage),

    /// Authentication response from server.
    AuthSuccess { user_id: String },

    /// Authentication error from server.
    AuthError { error: String },

    /// Server query sent to client.
    ServerQuery { id: String, kind: serde_json::Value },

    /// Client response to query.
    QueryResponse {
        query_id: String,
        result: serde_json::Value,
    },

    /// LLM streaming event.
    LlmEvent {
        event_type: String,
        data: serde_json::Value,
    },

    /// Keepalive ping.
    Ping { timestamp: i64 },

    /// Keepalive pong.
    Pong { timestamp: i64 },

    /// Connection control message.
    Control { command: String, payload: Option<serde_json::Value> },
}

/// Phase 3 WebSocket endpoint configuration.
pub mod config {
    use std::time::Duration;

    /// Ping interval for keepalive (30 seconds).
    pub const PING_INTERVAL: Duration = Duration::from_secs(30);

    /// Pong timeout (10 seconds after ping).
    pub const PONG_TIMEOUT: Duration = Duration::from_secs(10);

    /// Maximum message size (10 MB).
    pub const MAX_MESSAGE_SIZE: usize = 10 * 1024 * 1024;

    /// Maximum queued messages per connection.
    pub const MAX_QUEUE_SIZE: usize = 1000;
}

pub mod handler {
    use super::*;
    use crate::AppState;
    use axum::{
        extract::{
            ws::{Message, WebSocket},
            Path, State, WebSocketUpgrade,
        },
        response::IntoResponse,
    };
    use chrono::Utc;
    use futures::{SinkExt, StreamExt};
    use sha2::{Digest, Sha256};
    use std::sync::Arc;
    use tokio::sync::mpsc;
    use tracing::{debug, error};

    fn hash_token(token: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(token.as_bytes());
        hex::encode(hasher.finalize())
    }

    #[derive(Debug, Clone, serde::Deserialize)]
    pub struct WsQueryParams {
        pub protocol_version: Option<String>,
        pub features: Option<String>,
        pub reconnect_token: Option<String>,
    }

    #[utoipa::path(
        get,
        path = "/v1/ws/sessions/{session_id}",
        tag = "websocket",
        params(
            ("session_id" = String, Path, description = "Session ID for the WebSocket connection")
        ),
        responses(
            (status = 101, description = "WebSocket connection established"),
            (status = 401, description = "Unauthorized - invalid session or token"),
            (status = 400, description = "Bad request - invalid upgrade request"),
        )
    )]
    pub async fn ws_upgrade_handler(
        ws: WebSocketUpgrade,
        Path(session_id): Path<String>,
        State(state): State<AppState>,
        headers: http::HeaderMap,
    ) -> impl IntoResponse {
        info!(session_id = %session_id, "WebSocket upgrade request received");

        let session_repo = state.session_repo.clone();
        let user_repo = state.user_repo.clone();

        let cookie_auth = extract_cookie_auth(&headers, |session_token| {
            let session_repo = session_repo.clone();
            let user_repo = user_repo.clone();
            async move {
                validate_session_token(&session_token, &session_repo, &user_repo).await
            }
        })
        .await;

        let bearer_auth = extract_upgrade_bearer_token(&headers);

        ws.on_upgrade(move |socket| {
            handle_ws_connection(socket, session_id, state, cookie_auth, bearer_auth)
        })
    }

    #[tracing::instrument(skip(session_token, session_repo, user_repo))]
    async fn validate_session_token(
        session_token: &str,
        session_repo: &Arc<crate::db::SessionRepository>,
        user_repo: &Arc<crate::db::UserRepository>,
    ) -> Option<CurrentUser> {
        let token_hash = hash_token(session_token);

        let session = match session_repo.get_session_by_token_hash(&token_hash).await {
            Ok(Some(session)) => session,
            Ok(None) => {
                debug!("Session not found for token hash");
                return None;
            }
            Err(e) => {
                error!(error = %e, "Failed to look up session");
                return None;
            }
        };

        if session.expires_at < Utc::now() {
            debug!(session_id = %session.id, "Session expired");
            return None;
        }

        let user = match user_repo.get_user_by_id(&session.user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                warn!(user_id = %session.user_id, "User not found for valid session");
                return None;
            }
            Err(e) => {
                error!(error = %e, "Failed to look up user");
                return None;
            }
        };

        let session_id = session.id;
        let session_repo = session_repo.clone();
        tokio::spawn(async move {
            if let Err(e) = session_repo.update_session_last_used(&session_id).await {
                warn!(error = %e, "Failed to update session last used");
            }
        });

        Some(CurrentUser::from_session(user, session.id))
    }

    async fn handle_ws_connection(
        socket: WebSocket,
        session_id: String,
        state: AppState,
        cookie_auth: Option<CurrentUser>,
        bearer_auth: Option<String>,
    ) {
        let (mut sender, mut receiver) = socket.split();
        let session_id_for_log = session_id.clone();

        let conn = if let Some(user) = cookie_auth {
            info!(session_id = %session_id, user_id = %user.user.id, "WebSocket authenticated via cookie");
            WebSocketConnection::new_authenticated(session_id.clone(), user)
        } else if let Some(ref token) = bearer_auth {
            match validate_bearer_token(
                token,
                &state.session_repo,
                &state.api_key_repo,
                &state.user_repo,
            ).await {
                Some(user) => {
                    info!(session_id = %session_id, user_id = %user.user.id, "WebSocket authenticated via bearer token");
                    WebSocketConnection::new_authenticated(session_id.clone(), user)
                }
                None => {
                    info!(session_id = %session_id, "WebSocket awaiting first-message auth");
                    WebSocketConnection::new_unauthenticated(session_id.clone())
                }
            }
        } else {
            info!(session_id = %session_id, "WebSocket awaiting first-message auth");
            WebSocketConnection::new_unauthenticated(session_id.clone())
        };

        let conn = Arc::new(conn);
        let (tx, mut rx) = mpsc::channel::<WebSocketMessage>(config::MAX_QUEUE_SIZE);

        let conn_send = conn.clone();
        let send_task = tokio::spawn(async move {
            while let Some(msg) = rx.recv().await {
                let json = match serde_json::to_string(&msg) {
                    Ok(j) => j,
                    Err(e) => {
                        error!(error = %e, "Failed to serialize WebSocket message");
                        continue;
                    }
                };
                if let Err(e) = sender.send(Message::Text(json.into())).await {
                    debug!(error = %e, "Failed to send WebSocket message");
                    break;
                }
            }
            conn_send.close().await;
        });

        let conn_recv = conn.clone();
        let query_manager = state.query_manager.clone();
        let session_repo = state.session_repo.clone();
        let api_key_repo = state.api_key_repo.clone();
        let user_repo = state.user_repo.clone();
        let tx_clone = tx.clone();
        let session_id_recv = session_id;
        let recv_task = tokio::spawn(async move {
            let auth_deadline = tokio::time::Instant::now() + auth_timeout();
            let mut ping_interval = tokio::time::interval(config::PING_INTERVAL);
            ping_interval.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

            loop {
                tokio::select! {
                    _ = tokio::time::sleep_until(auth_deadline), if !conn_recv.is_authenticated().await => {
                        warn!(session_id = %session_id_recv, "WebSocket auth timeout");
                        let _ = tx_clone.send(WebSocketMessage::AuthError {
                            error: "authentication timeout".to_string(),
                        }).await;
                        conn_recv.handle_auth_timeout().await;
                        break;
                    }
                    _ = ping_interval.tick() => {
                        if conn_recv.is_authenticated().await {
                            let timestamp = chrono::Utc::now().timestamp();
                            let _ = tx_clone.send(WebSocketMessage::Ping { timestamp }).await;
                        }
                    }
                    msg = receiver.next() => {
                        match msg {
                            Some(Ok(Message::Text(text))) => {
                                let text_str: &str = &text;
                                if let Err(e) = handle_text_message(
                                    text_str,
                                    &conn_recv,
                                    &tx_clone,
                                    &query_manager,
                                    &session_repo,
                                    &api_key_repo,
                                    &user_repo,
                                ).await {
                                    debug!(error = %e, "Error handling WebSocket message");
                                }
                            }
                            Some(Ok(Message::Binary(data))) => {
                                if let Ok(text) = String::from_utf8(data.to_vec()) {
                                    if let Err(e) = handle_text_message(
                                        &text,
                                        &conn_recv,
                                        &tx_clone,
                                        &query_manager,
                                        &session_repo,
                                        &api_key_repo,
                                        &user_repo,
                                    ).await {
                                        debug!(error = %e, "Error handling WebSocket binary message");
                                    }
                                }
                            }
                            Some(Ok(Message::Ping(_data))) => {
                                let _ = tx_clone.send(WebSocketMessage::Pong {
                                    timestamp: chrono::Utc::now().timestamp(),
                                }).await;
                                debug!("Received ping, sending pong");
                            }
                            Some(Ok(Message::Pong(_))) => {
                                debug!("Received pong");
                            }
                            Some(Ok(Message::Close(_))) => {
                                info!(session_id = %session_id_recv, "WebSocket close received");
                                conn_recv.start_closing().await;
                                break;
                            }
                            Some(Err(e)) => {
                                debug!(error = %e, "WebSocket error");
                                break;
                            }
                            None => {
                                info!(session_id = %session_id_recv, "WebSocket connection closed");
                                break;
                            }
                        }
                    }
                }
            }
            conn_recv.close().await;
        });

        tokio::select! {
            _ = send_task => {}
            _ = recv_task => {}
        }

        info!(session_id = %session_id_for_log, "WebSocket connection terminated");
    }

    #[tracing::instrument(skip(token, session_repo, api_key_repo, user_repo))]
    async fn validate_bearer_token(
        token: &str,
        session_repo: &Arc<crate::db::SessionRepository>,
        api_key_repo: &Arc<crate::db::ApiKeyRepository>,
        user_repo: &Arc<crate::db::UserRepository>,
    ) -> Option<CurrentUser> {
        use loom_auth::middleware::identify_bearer_token;
        match identify_bearer_token(token) {
            BearerTokenType::AccessToken => {
                validate_access_token(token, session_repo, user_repo).await
            }
            BearerTokenType::ApiKey => {
                validate_api_key(token, api_key_repo, user_repo).await
            }
            BearerTokenType::Unknown => None,
        }
    }

    #[tracing::instrument(skip(token, session_repo, user_repo))]
    async fn validate_access_token(
        token: &str,
        session_repo: &Arc<crate::db::SessionRepository>,
        user_repo: &Arc<crate::db::UserRepository>,
    ) -> Option<CurrentUser> {
        let token_hash = hash_token(token);

        let (token_id, user_id) = match session_repo.get_access_token_by_hash(&token_hash).await {
            Ok(Some((id, uid))) => (id, uid),
            Ok(None) => {
                debug!("Access token not found for token hash");
                return None;
            }
            Err(e) => {
                error!(error = %e, "Failed to look up access token");
                return None;
            }
        };

        let user = match user_repo.get_user_by_id(&user_id).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                warn!(user_id = %user_id, "User not found for access token");
                return None;
            }
            Err(e) => {
                error!(error = %e, "Failed to look up user");
                return None;
            }
        };

        let session_repo = session_repo.clone();
        tokio::spawn(async move {
            if let Err(e) = session_repo.update_access_token_last_used(&token_id).await {
                warn!(error = %e, "Failed to update access token last used");
            }
        });

        Some(CurrentUser::from_access_token(user))
    }

    #[tracing::instrument(skip(token, api_key_repo, user_repo))]
    async fn validate_api_key(
        token: &str,
        api_key_repo: &Arc<crate::db::ApiKeyRepository>,
        user_repo: &Arc<crate::db::UserRepository>,
    ) -> Option<CurrentUser> {
        let token_hash = hash_token(token);

        let api_key = match api_key_repo.get_api_key_by_hash(&token_hash).await {
            Ok(Some(key)) => key,
            Ok(None) => {
                debug!("API key not found for token hash");
                return None;
            }
            Err(e) => {
                error!(error = %e, "Failed to look up API key");
                return None;
            }
        };

        if api_key.revoked_at.is_some() {
            debug!(api_key_id = %api_key.id, "API key is revoked");
            return None;
        }

        let user = match user_repo.get_user_by_id(&api_key.created_by).await {
            Ok(Some(user)) => user,
            Ok(None) => {
                warn!(user_id = %api_key.created_by, "User not found for API key");
                return None;
            }
            Err(e) => {
                error!(error = %e, "Failed to look up user");
                return None;
            }
        };

        let api_key_id = api_key.id.to_string();
        let api_key_repo = api_key_repo.clone();
        tokio::spawn(async move {
            if let Err(e) = api_key_repo.update_last_used(&api_key_id).await {
                warn!(error = %e, "Failed to update API key last used");
            }
        });

        Some(CurrentUser::from_api_key(user, api_key.id.into_inner()))
    }

    async fn handle_text_message(
        text: &str,
        conn: &Arc<WebSocketConnection>,
        tx: &mpsc::Sender<WebSocketMessage>,
        _query_manager: &Arc<crate::server_query::ServerQueryManager>,
        session_repo: &Arc<crate::db::SessionRepository>,
        api_key_repo: &Arc<crate::db::ApiKeyRepository>,
        user_repo: &Arc<crate::db::UserRepository>,
    ) -> Result<(), String> {
        let msg: WebSocketMessage = serde_json::from_str(text)
            .map_err(|e| format!("Invalid JSON: {e}"))?;

        match msg {
            WebSocketMessage::Auth(_auth_msg) => {
                if conn.is_authenticated().await {
                    let auth_state = conn.auth_state().await;
                    if let Some(ctx) = auth_state.user() {
                        let _ = tx.send(WebSocketMessage::AuthSuccess {
                            user_id: ctx.user_id().to_string(),
                        }).await;
                    }
                    return Ok(());
                }

                let session_repo = session_repo.clone();
                let api_key_repo = api_key_repo.clone();
                let user_repo = user_repo.clone();

                let result = conn.process_auth_message(text, |token, token_type| {
                    let session_repo = session_repo.clone();
                    let api_key_repo = api_key_repo.clone();
                    let user_repo = user_repo.clone();
                    async move {
                        use loom_auth::BearerTokenType;
                        match token_type {
                            BearerTokenType::AccessToken => {
                                validate_access_token(&token, &session_repo, &user_repo)
                                    .await
                                    .ok_or(WsAuthError::InvalidToken)
                            }
                            BearerTokenType::ApiKey => {
                                validate_api_key(&token, &api_key_repo, &user_repo)
                                    .await
                                    .ok_or(WsAuthError::InvalidApiKey)
                            }
                            BearerTokenType::Unknown => {
                                Err(WsAuthError::InvalidToken)
                            }
                        }
                    }
                }).await;

                match result {
                    Ok(response) => {
                        let _ = tx.send(WebSocketMessage::AuthSuccess {
                            user_id: response.user_id.unwrap_or_default(),
                        }).await;
                    }
                    Err(e) => {
                        let _ = tx.send(WebSocketMessage::AuthError {
                            error: e.to_string(),
                        }).await;
                    }
                }
            }
            WebSocketMessage::QueryResponse { query_id, result: _ } => {
                if !conn.is_authenticated().await {
                    let _ = tx.send(WebSocketMessage::AuthError {
                        error: "authentication required".to_string(),
                    }).await;
                    return Err("not authenticated".to_string());
                }

                debug!(query_id = %query_id, "Received query response via WebSocket");
            }
            WebSocketMessage::Pong { timestamp } => {
                debug!(timestamp = timestamp, "Received pong");
            }
            WebSocketMessage::Control { command, payload: _ } => {
                debug!(command = %command, "Received control message");
            }
            _ => {
                debug!("Received unexpected message type from client");
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_websocket_connection_unauthenticated() {
        let conn = WebSocketConnection::new_unauthenticated("session-123".to_string());
        assert_eq!(conn.session_id(), "session-123");
        assert_eq!(conn.connection_state().await, ConnectionState::Handshaking);
        assert!(!conn.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_websocket_connection_authenticated() {
        use chrono::Utc;
        use loom_auth::{Session, SessionType, User, UserId};

        let user = User {
        	id: UserId::generate(),
        	display_name: "Test User".to_string(),
        	primary_email: Some("test@example.com".to_string()),
        	avatar_url: None,
        	email_visible: true,
        	is_system_admin: false,
        	is_support: false,
        	is_auditor: false,
        	created_at: Utc::now(),
        	updated_at: Utc::now(),
        	deleted_at: None,
        	locale: None,
        };
        let session = Session::new(user.id, SessionType::Web);
        let current_user = CurrentUser::from_session(user, session.id);

        let conn = WebSocketConnection::new_authenticated("session-456".to_string(), current_user);
        assert_eq!(conn.session_id(), "session-456");
        assert_eq!(conn.connection_state().await, ConnectionState::Active);
        assert!(conn.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_connection_state_transitions() {
        let conn = WebSocketConnection::new_unauthenticated("session-789".to_string());

        assert_eq!(conn.connection_state().await, ConnectionState::Handshaking);

        conn.start_closing().await;
        assert_eq!(conn.connection_state().await, ConnectionState::Closing);

        conn.close().await;
        assert_eq!(conn.connection_state().await, ConnectionState::Closed);
    }

    #[tokio::test]
    async fn test_auth_time_remaining() {
        let conn = WebSocketConnection::new_unauthenticated("session-time".to_string());

        let remaining = conn.auth_time_remaining().await;
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() <= 30);
    }

    #[tokio::test]
    async fn test_process_auth_message_invalid_json() {
        let conn = WebSocketConnection::new_unauthenticated("session-invalid".to_string());

        let result = conn
            .process_auth_message("not valid json", |_, _| async {
                Ok::<_, WsAuthError>(CurrentUser::from_access_token(loom_auth::User {
                	id: loom_auth::UserId::generate(),
                	display_name: "Test".to_string(),
                	primary_email: None,
                	avatar_url: None,
                	email_visible: true,
                	is_system_admin: false,
                	is_support: false,
                	is_auditor: false,
                	created_at: chrono::Utc::now(),
                	updated_at: chrono::Utc::now(),
                	deleted_at: None,
                	locale: None,
                }))
                })
                .await;

                assert!(result.is_err());
                assert!(matches!(result.unwrap_err(), WsAuthError::InvalidAuthMessage));
                }

    #[tokio::test]
    async fn test_process_auth_message_invalid_format() {
        let conn = WebSocketConnection::new_unauthenticated("session-format".to_string());

        let result = conn
            .process_auth_message(r#"{"type":"auth","token":""}"#, |_, _| async {
                Ok::<_, WsAuthError>(CurrentUser::from_access_token(loom_auth::User {
                	id: loom_auth::UserId::generate(),
                	display_name: "Test".to_string(),
                	primary_email: None,
                	avatar_url: None,
                	email_visible: true,
                	is_system_admin: false,
                	is_support: false,
                	is_auditor: false,
                	created_at: chrono::Utc::now(),
                	updated_at: chrono::Utc::now(),
                	deleted_at: None,
                	locale: None,
                }))
                })
                .await;

        assert!(result.is_err());
        assert!(matches!(result.unwrap_err(), WsAuthError::InvalidAuthMessage));
    }

    #[tokio::test]
    async fn test_handle_auth_timeout() {
        let conn = WebSocketConnection::new_unauthenticated("session-timeout".to_string());

        let close_code = conn.handle_auth_timeout().await;
        assert_eq!(close_code, 4001);
        assert_eq!(conn.connection_state().await, ConnectionState::Closing);

        let auth_state = conn.auth_state().await;
        assert!(matches!(auth_state, WsAuthState::Failed(WsAuthError::Timeout)));
    }

    #[test]
    fn test_websocket_message_serialization() {
        let msg = WebSocketMessage::Ping {
            timestamp: 1234567890,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"ping\""));
        assert!(json.contains("\"timestamp\":1234567890"));

        let parsed: WebSocketMessage = serde_json::from_str(&json).unwrap();
        if let WebSocketMessage::Ping { timestamp } = parsed {
            assert_eq!(timestamp, 1234567890);
        } else {
            panic!("Expected Ping message");
        }
    }

    #[test]
    fn test_websocket_message_auth_serialization() {
        let auth_msg = loom_auth::WsAuthMessage::new("lt_test123");
        let msg = WebSocketMessage::Auth(auth_msg);
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"auth\""));
        assert!(json.contains("\"token\":\"lt_test123\""));
    }

    #[test]
    fn test_websocket_message_query_response_serialization() {
        let msg = WebSocketMessage::QueryResponse {
            query_id: "Q-abc123".to_string(),
            result: serde_json::json!({"content": "file contents"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"query_response\""));
        assert!(json.contains("\"query_id\":\"Q-abc123\""));

        let parsed: WebSocketMessage = serde_json::from_str(&json).unwrap();
        if let WebSocketMessage::QueryResponse { query_id, result } = parsed {
            assert_eq!(query_id, "Q-abc123");
            assert!(result.get("content").is_some());
        } else {
            panic!("Expected QueryResponse message");
        }
    }

    #[test]
    fn test_websocket_message_server_query_serialization() {
        let msg = WebSocketMessage::ServerQuery {
            id: "Q-def456".to_string(),
            kind: serde_json::json!({"type": "read_file", "path": "/src/main.rs"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"server_query\""));
        assert!(json.contains("\"id\":\"Q-def456\""));
    }

    #[test]
    fn test_websocket_message_llm_event_serialization() {
        let msg = WebSocketMessage::LlmEvent {
            event_type: "text_delta".to_string(),
            data: serde_json::json!({"text": "Hello"}),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"llm_event\""));
        assert!(json.contains("\"event_type\":\"text_delta\""));
    }

    #[test]
    fn test_websocket_message_control_serialization() {
        let msg = WebSocketMessage::Control {
            command: "close".to_string(),
            payload: Some(serde_json::json!({"reason": "shutdown"})),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"control\""));
        assert!(json.contains("\"command\":\"close\""));
    }

    #[test]
    fn test_websocket_message_auth_success_serialization() {
        let msg = WebSocketMessage::AuthSuccess {
            user_id: "U-abc123".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"auth_success\""));
        assert!(json.contains("\"user_id\":\"U-abc123\""));
    }

    #[test]
    fn test_websocket_message_auth_error_serialization() {
        let msg = WebSocketMessage::AuthError {
            error: "invalid token".to_string(),
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(json.contains("\"type\":\"auth_error\""));
        assert!(json.contains("\"error\":\"invalid token\""));
    }

    #[tokio::test]
    async fn test_authenticated_connection_has_user_context() {
        use chrono::Utc;
        use loom_auth::{Session, SessionType, User, UserId};

        let user_id = UserId::generate();
        let user = User {
        	id: user_id,
        	display_name: "Auth Test User".to_string(),
        	primary_email: Some("auth@example.com".to_string()),
        	avatar_url: None,
        	email_visible: true,
        	is_system_admin: false,
        	is_support: false,
        	is_auditor: false,
        	created_at: Utc::now(),
        	updated_at: Utc::now(),
        	deleted_at: None,
        	locale: None,
        };
        let session = Session::new(user.id, SessionType::Web);
        let current_user = CurrentUser::from_session(user, session.id);

        let conn = WebSocketConnection::new_authenticated("test-session".to_string(), current_user);

        let auth_state = conn.auth_state().await;
        assert!(auth_state.is_authenticated());

        if let WsAuthState::Authenticated(ctx) = auth_state {
            assert_eq!(ctx.auth_method, WsAuthMethod::SessionCookie);
            assert_eq!(*ctx.user_id(), user_id);
        } else {
            panic!("Expected authenticated state");
        }
    }

    #[tokio::test]
    async fn test_auth_timeout_not_elapsed_initially() {
        let conn = WebSocketConnection::new_unauthenticated("fresh-session".to_string());
        assert!(!conn.is_auth_timeout_elapsed());

        let remaining = conn.auth_time_remaining().await;
        assert!(remaining.is_some());
        assert!(remaining.unwrap().as_secs() >= 29);
    }

    #[tokio::test]
    async fn test_authenticated_connection_has_no_timeout() {
        use chrono::Utc;
        use loom_auth::{Session, SessionType, User, UserId};

        let user = User {
        	id: UserId::generate(),
        	display_name: "No Timeout User".to_string(),
        	primary_email: None,
        	avatar_url: None,
        	email_visible: true,
        	is_system_admin: false,
        	is_support: false,
        	is_auditor: false,
        	created_at: Utc::now(),
        	updated_at: Utc::now(),
        	deleted_at: None,
        	locale: None,
        };
        let session = Session::new(user.id, SessionType::Cli);
        let current_user = CurrentUser::from_session(user, session.id);

        let conn = WebSocketConnection::new_authenticated("authed-session".to_string(), current_user);

        let remaining = conn.auth_time_remaining().await;
        assert!(remaining.is_none());
    }

    #[tokio::test]
    async fn test_process_valid_auth_message() {
        use chrono::Utc;
        use loom_auth::{User, UserId};

        let conn = WebSocketConnection::new_unauthenticated("auth-valid".to_string());
        let test_user_id = UserId::generate();

        let result = conn
            .process_auth_message(
                r#"{"type":"auth","token":"lt_validtoken123"}"#,
                |_token, _token_type| async move {
                    Ok(CurrentUser::from_access_token(User {
                    	id: test_user_id,
                    	display_name: "Valid User".to_string(),
                    	primary_email: None,
                    	avatar_url: None,
                    	email_visible: true,
                    	is_system_admin: false,
                    	is_support: false,
                    	is_auditor: false,
                    	created_at: Utc::now(),
                    	updated_at: Utc::now(),
                    	deleted_at: None,
                    	locale: None,
                    }))
                },
            )
            .await;

        assert!(result.is_ok());
        let response = result.unwrap();
        assert!(response.success);
        assert!(response.user_id.is_some());
        assert!(conn.is_authenticated().await);
    }

    #[tokio::test]
    async fn test_process_auth_message_with_unknown_token_type() {
        let conn = WebSocketConnection::new_unauthenticated("unknown-token".to_string());

        let result = conn
            .process_auth_message(
                r#"{"type":"auth","token":"unknownprefix_token123"}"#,
                |_token, _token_type| async move {
                    Err(WsAuthError::InvalidToken)
                },
            )
            .await;

        assert!(result.is_err());
        assert!(!conn.is_authenticated().await);
    }

    mod handler_tests {
        use super::*;

        #[test]
        fn test_ws_query_params_deserialize() {
            let json = r#"{"protocol_version":"3.0","features":"compression,ack"}"#;
            let params: handler::WsQueryParams = serde_json::from_str(json).unwrap();
            assert_eq!(params.protocol_version, Some("3.0".to_string()));
            assert_eq!(params.features, Some("compression,ack".to_string()));
            assert!(params.reconnect_token.is_none());
        }

        #[test]
        fn test_ws_query_params_empty() {
            let json = r#"{}"#;
            let params: handler::WsQueryParams = serde_json::from_str(json).unwrap();
            assert!(params.protocol_version.is_none());
            assert!(params.features.is_none());
            assert!(params.reconnect_token.is_none());
        }
    }

    mod config_tests {
        use super::*;

        #[test]
        fn test_ping_interval() {
            assert_eq!(config::PING_INTERVAL.as_secs(), 30);
        }

        #[test]
        fn test_pong_timeout() {
            assert_eq!(config::PONG_TIMEOUT.as_secs(), 10);
        }

        #[test]
        fn test_max_message_size() {
            assert_eq!(config::MAX_MESSAGE_SIZE, 10 * 1024 * 1024);
        }

        #[test]
        fn test_max_queue_size() {
            assert_eq!(config::MAX_QUEUE_SIZE, 1000);
        }
    }

    mod auth_error_tests {
        use super::*;

        #[tokio::test]
        async fn test_process_auth_with_token_expired_error() {
            let conn = WebSocketConnection::new_unauthenticated("expired-token".to_string());

            let result = conn
                .process_auth_message(
                    r#"{"type":"auth","token":"lt_expiredtoken"}"#,
                    |_token, _token_type| async move {
                        Err(WsAuthError::TokenExpired)
                    },
                )
                .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), WsAuthError::TokenExpired));
            assert!(!conn.is_authenticated().await);
        }

        #[tokio::test]
        async fn test_process_auth_with_token_revoked_error() {
            let conn = WebSocketConnection::new_unauthenticated("revoked-token".to_string());

            let result = conn
                .process_auth_message(
                    r#"{"type":"auth","token":"lt_revokedtoken"}"#,
                    |_token, _token_type| async move {
                        Err(WsAuthError::TokenRevoked)
                    },
                )
                .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), WsAuthError::TokenRevoked));
        }

        #[tokio::test]
        async fn test_process_auth_with_api_key_revoked_error() {
            let conn = WebSocketConnection::new_unauthenticated("revoked-api-key".to_string());

            let result = conn
                .process_auth_message(
                    r#"{"type":"auth","token":"lk_revokedapikey"}"#,
                    |_token, _token_type| async move {
                        Err(WsAuthError::ApiKeyRevoked)
                    },
                )
                .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), WsAuthError::ApiKeyRevoked));
        }

        #[tokio::test]
        async fn test_process_auth_with_user_inactive_error() {
            let conn = WebSocketConnection::new_unauthenticated("inactive-user".to_string());

            let result = conn
                .process_auth_message(
                    r#"{"type":"auth","token":"lt_inactiveuser"}"#,
                    |_token, _token_type| async move {
                        Err(WsAuthError::UserInactive)
                    },
                )
                .await;

            assert!(result.is_err());
            assert!(matches!(result.unwrap_err(), WsAuthError::UserInactive));
        }

        #[tokio::test]
        async fn test_already_authenticated_returns_success() {
            use chrono::Utc;
            use loom_auth::{Session, SessionType, User, UserId};

            let user_id = UserId::generate();
            let user = User {
            	id: user_id,
            	display_name: "Already Auth User".to_string(),
            	primary_email: None,
            	avatar_url: None,
            	email_visible: true,
            	is_system_admin: false,
            	is_support: false,
            	is_auditor: false,
            	created_at: Utc::now(),
            	updated_at: Utc::now(),
            	deleted_at: None,
            	locale: None,
            };
            let session = Session::new(user.id, SessionType::Web);
            let current_user = CurrentUser::from_session(user, session.id);

            let conn = WebSocketConnection::new_authenticated("already-auth".to_string(), current_user);

            let result = conn
                .process_auth_message(
                    r#"{"type":"auth","token":"lt_anothertoken"}"#,
                    |_token, _token_type| async move {
                        panic!("Should not call validator for already authenticated connection");
                    },
                )
                .await;

            assert!(result.is_ok());
            let response = result.unwrap();
            assert!(response.success);
            assert!(response.user_id.is_some());
        }
    }
}
