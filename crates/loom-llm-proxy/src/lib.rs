//! LLM proxy client for communicating with server-side LLM proxies.
//!
//! This crate provides a client implementation that talks to server proxy
//! endpoints instead of direct LLM provider APIs. The client does not need
//! API keys as authentication is handled server-side.
//!
//! # Example
//!
//! ```no_run
//! use loom_llm_proxy::ProxyLlmClient;
//! use loom_core::{LlmClient, LlmRequest, Message};
//!
//! # async fn example() -> Result<(), Box<dyn std::error::Error>> {
//! let client = ProxyLlmClient::new("http://localhost:8080");
//!
//! let request = LlmRequest::new("claude-3-5-sonnet-20241022")
//!     .with_messages(vec![Message::user("Hello!")]);
//!
//! let response = client.complete(request).await?;
//! println!("Response: {}", response.message.content);
//! # Ok(())
//! # }
//! ```

pub mod client;
pub mod stream;
pub mod types;

pub use client::ProxyLlmClient;
pub use stream::ProxyLlmStream;
pub use types::{LlmProxyResponse, LlmStreamEvent};
