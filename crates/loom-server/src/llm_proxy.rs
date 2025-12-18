//! LLM proxy handlers for server-side LLM completion requests.

use axum::{
    extract::State,
    http::StatusCode,
    response::{
        sse::{Event, Sse},
        IntoResponse,
    },
    Json,
};
use loom_core::{LlmError, LlmEvent, LlmRequest, Message, ToolCall, Usage};
use serde::{Deserialize, Serialize};
use std::convert::Infallible;
use tokio_stream::wrappers::ReceiverStream;

use crate::{api::AppState, error::ServerError};

/// Wire format for LLM streaming events sent over SSE.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LlmStreamEvent {
    TextDelta { content: String },
    ToolCallDelta {
        call_id: String,
        tool_name: String,
        arguments_fragment: String,
    },
    Completed { response: LlmProxyResponse },
    Error { message: String },
}

/// Wire format for LLM response, serializable for proxy communication.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmProxyResponse {
    pub message: Message,
    pub tool_calls: Vec<ToolCall>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub usage: Option<Usage>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub finish_reason: Option<String>,
}

impl From<loom_core::LlmResponse> for LlmProxyResponse {
    fn from(response: loom_core::LlmResponse) -> Self {
        Self {
            message: response.message,
            tool_calls: response.tool_calls,
            usage: response.usage,
            finish_reason: response.finish_reason,
        }
    }
}

/// POST /proxy/llm/complete - Synchronous LLM completion.
#[axum::debug_handler]
pub async fn proxy_llm_complete(
    State(state): State<AppState>,
    Json(request): Json<LlmRequest>,
) -> Result<impl IntoResponse, ServerError> {
    let service = state.llm_service.as_ref().ok_or_else(|| {
        tracing::error!("proxy_llm_complete: LLM service not configured");
        ServerError::ServiceUnavailable("LLM service is not configured on the server".into())
    })?;

    tracing::debug!(
        model = %request.model,
        message_count = request.messages.len(),
        tool_count = request.tools.len(),
        "proxy_llm_complete: sending request"
    );

    let response = service.complete(request).await.map_err(map_llm_error)?;

    tracing::info!(
        finish_reason = ?response.finish_reason,
        tool_call_count = response.tool_calls.len(),
        "proxy_llm_complete: returning response"
    );

    Ok((StatusCode::OK, Json(LlmProxyResponse::from(response))))
}

/// POST /proxy/llm/stream - Streaming LLM completion via SSE.
#[axum::debug_handler]
pub async fn proxy_llm_stream(
    State(state): State<AppState>,
    Json(request): Json<LlmRequest>,
) -> Result<Sse<impl futures::Stream<Item = Result<Event, Infallible>>>, ServerError> {
    let service = state.llm_service.as_ref().ok_or_else(|| {
        tracing::error!("proxy_llm_stream: LLM service not configured");
        ServerError::ServiceUnavailable("LLM service is not configured on the server".into())
    })?;

    tracing::debug!(
        model = %request.model,
        message_count = request.messages.len(),
        tool_count = request.tools.len(),
        "proxy_llm_stream: starting stream"
    );

    let stream = service.complete_streaming(request).await.map_err(map_llm_error)?;

    let (tx, rx) = tokio::sync::mpsc::channel::<Result<Event, Infallible>>(32);

    tokio::spawn(async move {
        let mut stream = stream;
        while let Some(event) = stream.next().await {
            let sse_event = match event {
                LlmEvent::TextDelta { content } => {
                    let stream_event = LlmStreamEvent::TextDelta { content };
                    match serde_json::to_string(&stream_event) {
                        Ok(json) => Event::default().data(json),
                        Err(e) => {
                            tracing::error!(error = %e, "failed to serialize text delta");
                            continue;
                        }
                    }
                }
                LlmEvent::ToolCallDelta {
                    call_id,
                    tool_name,
                    arguments_fragment,
                } => {
                    let stream_event = LlmStreamEvent::ToolCallDelta {
                        call_id,
                        tool_name,
                        arguments_fragment,
                    };
                    match serde_json::to_string(&stream_event) {
                        Ok(json) => Event::default().data(json),
                        Err(e) => {
                            tracing::error!(error = %e, "failed to serialize tool call delta");
                            continue;
                        }
                    }
                }
                LlmEvent::Completed(response) => {
                    tracing::info!(
                        finish_reason = ?response.finish_reason,
                        tool_call_count = response.tool_calls.len(),
                        "proxy_llm_stream: stream completed"
                    );
                    let stream_event = LlmStreamEvent::Completed {
                        response: LlmProxyResponse::from(response),
                    };
                    match serde_json::to_string(&stream_event) {
                        Ok(json) => Event::default().data(json),
                        Err(e) => {
                            tracing::error!(error = %e, "failed to serialize completed event");
                            continue;
                        }
                    }
                }
                LlmEvent::Error(err) => {
                    tracing::warn!(error = %err, "proxy_llm_stream: stream error");
                    let stream_event = LlmStreamEvent::Error {
                        message: err.to_string(),
                    };
                    match serde_json::to_string(&stream_event) {
                        Ok(json) => Event::default().data(json),
                        Err(e) => {
                            tracing::error!(error = %e, "failed to serialize error event");
                            continue;
                        }
                    }
                }
            };

            if tx.send(Ok(sse_event)).await.is_err() {
                tracing::debug!("proxy_llm_stream: client disconnected");
                break;
            }
        }
    });

    Ok(Sse::new(ReceiverStream::new(rx)))
}

/// Map LlmError to ServerError for HTTP response conversion.
pub fn map_llm_error(err: LlmError) -> ServerError {
    match err {
        LlmError::Http(msg) => {
            tracing::error!(error = %msg, "LLM HTTP error");
            ServerError::UpstreamError(format!("LLM HTTP error: {}", msg))
        }
        LlmError::Api(msg) => {
            tracing::warn!(error = %msg, "LLM API error");
            ServerError::UpstreamError(format!("LLM API error: {}", msg))
        }
        LlmError::Timeout => {
            tracing::warn!("LLM request timed out");
            ServerError::UpstreamTimeout("LLM request timed out".into())
        }
        LlmError::InvalidResponse(msg) => {
            tracing::error!(error = %msg, "Invalid LLM response");
            ServerError::UpstreamError(format!("Invalid LLM response: {}", msg))
        }
        LlmError::RateLimited { retry_after_secs } => {
            tracing::warn!(retry_after = ?retry_after_secs, "LLM rate limited");
            let msg = match retry_after_secs {
                Some(secs) => format!("LLM rate limited; retry after {} seconds", secs),
                None => "LLM rate limited; try again later".to_string(),
            };
            ServerError::ServiceUnavailable(msg)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use loom_core::Role;
    use proptest::prelude::*;

    proptest! {
        /// Validates that LlmStreamEvent text_delta events serialize correctly,
        /// ensuring SSE communication preserves content integrity.
        #[test]
        fn text_delta_serialization_roundtrip(content in "[a-zA-Z0-9 ]{0,100}") {
            let event = LlmStreamEvent::TextDelta { content: content.clone() };

            let json = serde_json::to_string(&event).expect("serialization should succeed");
            let deserialized: LlmStreamEvent = serde_json::from_str(&json)
                .expect("deserialization should succeed");

            match deserialized {
                LlmStreamEvent::TextDelta { content: deserialized_content } => {
                    prop_assert_eq!(content, deserialized_content);
                }
                _ => prop_assert!(false, "expected TextDelta variant"),
            }
        }

        /// Validates that LlmStreamEvent tool_call_delta events serialize correctly,
        /// ensuring tool invocation data is preserved during proxy communication.
        #[test]
        fn tool_call_delta_serialization_roundtrip(
            call_id in "[a-zA-Z0-9_-]{1,32}",
            tool_name in "[a-zA-Z_][a-zA-Z0-9_]{0,30}",
            arguments_fragment in "\\{[a-zA-Z0-9:,\" ]{0,50}\\}?",
        ) {
            let event = LlmStreamEvent::ToolCallDelta {
                call_id: call_id.clone(),
                tool_name: tool_name.clone(),
                arguments_fragment: arguments_fragment.clone(),
            };

            let json = serde_json::to_string(&event).expect("serialization should succeed");
            let deserialized: LlmStreamEvent = serde_json::from_str(&json)
                .expect("deserialization should succeed");

            match deserialized {
                LlmStreamEvent::ToolCallDelta {
                    call_id: d_call_id,
                    tool_name: d_tool_name,
                    arguments_fragment: d_args,
                } => {
                    prop_assert_eq!(call_id, d_call_id);
                    prop_assert_eq!(tool_name, d_tool_name);
                    prop_assert_eq!(arguments_fragment, d_args);
                }
                _ => prop_assert!(false, "expected ToolCallDelta variant"),
            }
        }

        /// Validates that LlmProxyResponse serialization preserves all fields,
        /// critical for accurate response forwarding through the proxy.
        #[test]
        fn proxy_response_serialization_roundtrip(
            content in "[a-zA-Z0-9 ]{0,100}",
            input_tokens in 0u32..1_000_000,
            output_tokens in 0u32..1_000_000,
        ) {
            let response = LlmProxyResponse {
                message: Message {
                    role: Role::Assistant,
                    content,
                    tool_call_id: None,
                    name: None,
                },
                tool_calls: vec![],
                usage: Some(Usage { input_tokens, output_tokens }),
                finish_reason: Some("stop".to_string()),
            };

            let json = serde_json::to_string(&response).expect("serialization should succeed");
            let deserialized: LlmProxyResponse = serde_json::from_str(&json)
                .expect("deserialization should succeed");

            prop_assert_eq!(response.message.content, deserialized.message.content);
            prop_assert_eq!(response.usage.as_ref().unwrap().input_tokens, deserialized.usage.as_ref().unwrap().input_tokens);
            prop_assert_eq!(response.usage.as_ref().unwrap().output_tokens, deserialized.usage.as_ref().unwrap().output_tokens);
        }
    }
}
