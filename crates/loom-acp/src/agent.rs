//! ACP Agent trait implementation for Loom.

use std::cell::RefCell;
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use agent_client_protocol::{
    self as acp, AgentCapabilities, AuthenticateRequest, AuthenticateResponse, CancelNotification,
    ContentChunk, ExtNotification, ExtRequest, ExtResponse, Implementation, InitializeRequest,
    InitializeResponse, LoadSessionRequest, LoadSessionResponse, NewSessionRequest,
    NewSessionResponse, PromptRequest, PromptResponse, ProtocolVersion, SessionId,
    SessionNotification, SessionUpdate, SetSessionModeRequest, SetSessionModeResponse, StopReason,
};
use loom_core::{LlmClient, LlmEvent, LlmRequest, Message, ToolCall, ToolContext, ToolDefinition};
use loom_thread::{
    AgentStateKind, AgentStateSnapshot, MessageRole, MessageSnapshot, Thread, ThreadId,
    ThreadStore, ToolCallSnapshot,
};
use loom_tools::ToolRegistry;
use serde_json::value::RawValue;
use tokio::sync::{mpsc, oneshot};
use tracing::{debug, error, info, instrument, warn};

use crate::error::AcpError;
use crate::session::{SessionNotificationRequest, SessionState};

/// Loom's implementation of the ACP Agent trait.
///
/// This struct bridges ACP protocol messages to Loom's existing infrastructure:
/// - LLM client for completions
/// - Tool registry for tool execution  
/// - Thread store for persistence
pub struct LoomAcpAgent {
    /// LLM client for completions
    llm_client: Arc<dyn LlmClient>,

    /// Tool registry for tool execution
    tools: Arc<ToolRegistry>,

    /// Tool definitions (cached for LLM requests)
    tool_definitions: Vec<ToolDefinition>,

    /// Thread persistence
    thread_store: Arc<dyn ThreadStore>,

    /// Default workspace root for new sessions
    default_workspace_root: PathBuf,

    /// Default provider name (e.g., "anthropic")
    provider: String,

    /// Channel to send session notifications to the ACP connection
    session_update_tx: mpsc::UnboundedSender<SessionNotificationRequest>,

    /// Active sessions keyed by SessionId
    /// Uses RefCell because ACP Agent trait is ?Send (single-threaded)
    sessions: RefCell<HashMap<String, SessionState>>,
}

impl std::fmt::Debug for LoomAcpAgent {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LoomAcpAgent")
            .field("default_workspace_root", &self.default_workspace_root)
            .field("provider", &self.provider)
            .field("session_count", &self.sessions.borrow().len())
            .finish()
    }
}

impl LoomAcpAgent {
    /// Create a new Loom ACP agent.
    pub fn new(
        llm_client: Arc<dyn LlmClient>,
        tools: Arc<ToolRegistry>,
        thread_store: Arc<dyn ThreadStore>,
        default_workspace_root: PathBuf,
        provider: String,
        session_update_tx: mpsc::UnboundedSender<SessionNotificationRequest>,
    ) -> Self {
        let tool_definitions = tools.definitions();

        Self {
            llm_client,
            tools,
            tool_definitions,
            thread_store,
            default_workspace_root,
            provider,
            session_update_tx,
            sessions: RefCell::new(HashMap::new()),
        }
    }

    /// Send a text chunk notification to the client.
    async fn send_message_chunk(
        &self,
        session_id: &SessionId,
        text: String,
    ) -> Result<(), AcpError> {
        let notification = SessionNotification::new(
            session_id.clone(),
            SessionUpdate::AgentMessageChunk(ContentChunk::new(text.into())),
        );

        let (tx, rx) = oneshot::channel();
        self.session_update_tx
            .send(SessionNotificationRequest {
                notification,
                completion_tx: tx,
            })
            .map_err(|_| AcpError::NotificationChannelClosed)?;

        rx.await.map_err(|_| AcpError::NotificationChannelClosed)?;

        Ok(())
    }

    /// Execute a tool and return the result as a Message.
    #[instrument(skip(self, ctx))]
    async fn execute_tool(&self, call: &ToolCall, ctx: &ToolContext) -> Message {
        debug!(
            tool_id = %call.id,
            tool_name = %call.tool_name,
            "executing tool"
        );

        let result = match self.tools.get(&call.tool_name) {
            Some(tool) => match tool.invoke(call.arguments_json.clone(), ctx).await {
                Ok(output) => {
                    debug!(tool_id = %call.id, "tool succeeded");
                    serde_json::to_string(&output).unwrap_or_else(|_| "{}".to_string())
                }
                Err(e) => {
                    warn!(tool_id = %call.id, error = %e, "tool failed");
                    format!("Error: {}", e)
                }
            },
            None => {
                warn!(tool_id = %call.id, tool_name = %call.tool_name, "tool not found");
                format!("Error: tool '{}' not found", call.tool_name)
            }
        };

        Message::tool(&call.id, &call.tool_name, result)
    }

    /// Run the prompt loop: call LLM, execute tools, repeat until done.
    #[instrument(skip(self, session))]
    async fn run_prompt_loop(&self, session: &mut SessionState) -> Result<StopReason, AcpError> {
        loop {
            // Check cancellation
            if session.is_cancelled() {
                info!(session_id = %session.session_id, "prompt cancelled");
                return Ok(StopReason::Cancelled);
            }

            // Build LLM request
            let request = LlmRequest::new("default")
                .with_messages(session.messages.clone())
                .with_tools(self.tool_definitions.clone());

            debug!(
                session_id = %session.session_id,
                message_count = session.messages.len(),
                "calling LLM"
            );

            // Stream LLM response
            let mut stream = self
                .llm_client
                .complete_streaming(request)
                .await
                .map_err(AcpError::Llm)?;

            let mut assistant_content = String::new();
            let mut tool_calls: Vec<ToolCall> = Vec::new();

            while let Some(event) = stream.next().await {
                // Check cancellation during streaming
                if session.is_cancelled() {
                    info!(session_id = %session.session_id, "prompt cancelled during streaming");
                    return Ok(StopReason::Cancelled);
                }

                match event {
                    LlmEvent::TextDelta { content } => {
                        assistant_content.push_str(&content);
                        self.send_message_chunk(&session.session_id, content)
                            .await?;
                    }
                    LlmEvent::ToolCallDelta {
                        call_id,
                        tool_name,
                        arguments_fragment,
                    } => {
                        debug!(
                            call_id = %call_id,
                            tool_name = %tool_name,
                            fragment_len = arguments_fragment.len(),
                            "tool call delta"
                        );
                    }
                    LlmEvent::Completed(response) => {
                        debug!(
                            finish_reason = ?response.finish_reason,
                            tool_call_count = response.tool_calls.len(),
                            "LLM response complete"
                        );
                        tool_calls = response.tool_calls;
                        if !response.message.content.is_empty() {
                            assistant_content = response.message.content;
                        }
                        break;
                    }
                    LlmEvent::Error(e) => {
                        error!(error = ?e, "LLM stream error");
                        return Err(AcpError::Llm(e));
                    }
                }
            }

            // Add assistant message to conversation
            session
                .messages
                .push(Message::assistant(&assistant_content));

            // Persist assistant message to thread
            session.thread.conversation.messages.push(MessageSnapshot {
                role: MessageRole::Assistant,
                content: assistant_content.clone(),
                tool_call_id: None,
                tool_name: None,
                tool_calls: if tool_calls.is_empty() {
                    None
                } else {
                    Some(
                        tool_calls
                            .iter()
                            .map(|tc| ToolCallSnapshot {
                                id: tc.id.clone(),
                                tool_name: tc.tool_name.clone(),
                                arguments_json: tc.arguments_json.clone(),
                            })
                            .collect(),
                    )
                },
            });

            // If no tool calls, turn is complete
            if tool_calls.is_empty() {
                info!(session_id = %session.session_id, "turn complete - no tool calls");
                return Ok(StopReason::EndTurn);
            }

            // Execute tools
            let ctx = ToolContext {
                workspace_root: session.workspace_root.clone(),
            };

            for call in &tool_calls {
                debug!(
                    session_id = %session.session_id,
                    tool_id = %call.id,
                    tool_name = %call.tool_name,
                    "executing tool"
                );

                let tool_result = self.execute_tool(call, &ctx).await;

                // Add tool result to conversation
                session.messages.push(tool_result.clone());

                // Persist tool result to thread
                session.thread.conversation.messages.push(MessageSnapshot {
                    role: MessageRole::Tool,
                    content: tool_result.content,
                    tool_call_id: Some(call.id.clone()),
                    tool_name: Some(call.tool_name.clone()),
                    tool_calls: None,
                });
            }

            // Loop continues - LLM will process tool results
        }
    }

    /// Get a mutable reference to a session, cloning it out to avoid borrow issues.
    fn get_session(&self, session_id: &SessionId) -> Option<SessionState> {
        // We need to remove and return the session to avoid holding the borrow across await
        self.sessions.borrow_mut().remove(&session_id.to_string())
    }

    /// Put a session back after processing.
    fn put_session(&self, session: SessionState) {
        self.sessions
            .borrow_mut()
            .insert(session.session_id.to_string(), session);
    }
}

#[async_trait::async_trait(?Send)]
impl acp::Agent for LoomAcpAgent {
    #[instrument(skip(self, req))]
    async fn initialize(&self, req: InitializeRequest) -> acp::Result<InitializeResponse> {
        info!(
            client_version = %req.protocol_version,
            client_info = ?req.client_info,
            "ACP initialize request"
        );

        let mut capabilities = AgentCapabilities::default();
        capabilities.load_session = true;

        let agent_info = Implementation::new("loom", env!("CARGO_PKG_VERSION"))
            .title("Loom AI Coding Assistant");

        Ok(InitializeResponse::new(ProtocolVersion::V1)
            .agent_capabilities(capabilities)
            .agent_info(agent_info))
    }

    #[instrument(skip(self, _req))]
    async fn authenticate(&self, _req: AuthenticateRequest) -> acp::Result<AuthenticateResponse> {
        debug!("ACP authenticate request (no-op)");
        Ok(AuthenticateResponse::default())
    }

    #[instrument(skip(self, req))]
    async fn new_session(&self, req: NewSessionRequest) -> acp::Result<NewSessionResponse> {
        info!(cwd = ?req.cwd, "ACP new_session request");

        // Determine workspace root from request or default
        let workspace_root = req.cwd.clone();

        // Create new thread
        let mut thread = Thread::new();
        thread.workspace_root = Some(workspace_root.display().to_string());
        thread.cwd = Some(workspace_root.display().to_string());
        thread.loom_version = Some(env!("CARGO_PKG_VERSION").to_string());
        thread.provider = Some(self.provider.clone());

        // Persist thread
        self.thread_store
            .save(&thread)
            .await
            .map_err(AcpError::ThreadStore)?;

        // Create session ID from thread ID
        let session_id = SessionId::new(thread.id.to_string());

        // Create session state
        let session = SessionState::new(session_id.clone(), thread, workspace_root);

        // Store session
        self.sessions
            .borrow_mut()
            .insert(session_id.to_string(), session);

        info!(session_id = %session_id, "created new session");

        Ok(NewSessionResponse::new(session_id))
    }

    #[instrument(skip(self, req))]
    async fn load_session(&self, req: LoadSessionRequest) -> acp::Result<LoadSessionResponse> {
        info!(session_id = %req.session_id, "ACP load_session request");

        // Parse session ID as thread ID
        let thread_id = ThreadId::from_string(req.session_id.to_string());

        // Load thread from store
        let thread = self
            .thread_store
            .load(&thread_id)
            .await
            .map_err(AcpError::ThreadStore)?
            .ok_or_else(|| AcpError::SessionNotFound(req.session_id.to_string()))?;

        // Determine workspace root
        let workspace_root = thread
            .workspace_root
            .as_ref()
            .map(PathBuf::from)
            .unwrap_or_else(|| self.default_workspace_root.clone());

        // Create session state (rebuilds messages from thread)
        let session = SessionState::new(req.session_id.clone(), thread, workspace_root);

        info!(
            session_id = %req.session_id,
            message_count = session.messages.len(),
            "loaded session"
        );

        // Store session
        self.sessions
            .borrow_mut()
            .insert(req.session_id.to_string(), session);

        Ok(LoadSessionResponse::default())
    }

    #[instrument(skip(self, req))]
    async fn prompt(&self, req: PromptRequest) -> acp::Result<PromptResponse> {
        info!(
            session_id = %req.session_id,
            prompt_blocks = req.prompt.len(),
            "ACP prompt request"
        );

        // Get session (removes from map to avoid borrow across await)
        let mut session = self
            .get_session(&req.session_id)
            .ok_or_else(|| AcpError::SessionNotFound(req.session_id.to_string()))?;

        // Convert ACP ContentBlocks to Loom Message
        let user_text: String = req
            .prompt
            .iter()
            .filter_map(|block| {
                if let acp::ContentBlock::Text(t) = block {
                    Some(t.text.as_str())
                } else {
                    None
                }
            })
            .collect::<Vec<_>>()
            .join("\n");

        let user_message = Message::user(&user_text);
        session.messages.push(user_message.clone());

        // Persist user message to thread
        session.thread.conversation.messages.push(MessageSnapshot {
            role: MessageRole::User,
            content: user_text,
            tool_call_id: None,
            tool_name: None,
            tool_calls: None,
        });

        // Run prompt loop
        let stop_reason = match self.run_prompt_loop(&mut session).await {
            Ok(reason) => reason,
            Err(e) => {
                error!(error = ?e, "prompt loop failed");
                // Put session back before returning error
                self.put_session(session);
                return Err(e.into());
            }
        };

        // Update thread state and persist
        session.thread.agent_state = AgentStateSnapshot {
            kind: AgentStateKind::WaitingForUserInput,
            retries: 0,
            last_error: None,
            pending_tool_calls: Vec::new(),
        };
        session.thread.touch();

        if let Err(e) = self.thread_store.save(&session.thread).await {
            warn!(error = %e, "failed to persist thread after prompt");
        }

        // Put session back
        self.put_session(session);

        info!(stop_reason = ?stop_reason, "prompt complete");

        Ok(PromptResponse::new(stop_reason))
    }

    #[instrument(skip(self, req))]
    async fn cancel(&self, req: CancelNotification) -> acp::Result<()> {
        info!(session_id = %req.session_id, "ACP cancel request");

        if let Some(session) = self.sessions.borrow().get(&req.session_id.to_string()) {
            session.cancel();
        }

        Ok(())
    }

    async fn set_session_mode(
        &self,
        _req: SetSessionModeRequest,
    ) -> acp::Result<SetSessionModeResponse> {
        debug!("set_session_mode not implemented");
        Err(acp::Error::method_not_found())
    }

    async fn ext_method(&self, req: ExtRequest) -> acp::Result<ExtResponse> {
        use std::sync::Arc;
        debug!(method = %req.method, "unhandled extension method");
        let raw = RawValue::from_string("null".into())?;
        Ok(ExtResponse::new(Arc::from(raw)))
    }

    async fn ext_notification(&self, req: ExtNotification) -> acp::Result<()> {
        debug!(method = %req.method, "unhandled extension notification");
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// **Property: SessionId and ThreadId are interchangeable**
    ///
    /// Why this is important: ACP sessions are backed by Loom threads.
    /// The IDs must round-trip cleanly for session persistence to work.
    #[test]
    fn test_session_thread_id_mapping() {
        let thread = Thread::new();
        let thread_id_str = thread.id.to_string();

        // SessionId from ThreadId
        let session_id = SessionId::new(thread_id_str.clone());

        // Back to ThreadId
        let back = ThreadId::from_string(session_id.to_string());

        assert_eq!(thread.id.as_str(), back.as_str());
    }
}
