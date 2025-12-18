# Loom Architecture

## Overview

Loom is an AI-powered coding assistant built in Rust. It provides a REPL interface for interacting with LLM-powered agents that can execute tools to perform file system operations and other tasks.

The system is designed around three core principles:
1. **Modularity** - Clean separation between core abstractions, LLM providers, and tools
2. **Extensibility** - Easy addition of new LLM providers and tools via trait implementations
3. **Reliability** - Robust error handling with retry mechanisms and structured logging

## Crate Structure

Loom is organized as a Cargo workspace with 9 crates:

```
loom/
├── crates/
│   ├── loom-core/           # Core abstractions and types
│   ├── loom-http-retry/     # HTTP retry utilities
│   ├── loom-llm-anthropic/  # Anthropic Claude provider (server-only)
│   ├── loom-llm-openai/     # OpenAI provider (server-only)
│   ├── loom-llm-service/    # Server-side provider abstraction (owns API keys)
│   ├── loom-llm-proxy/      # Client-side HTTP LlmClient
│   ├── loom-server/         # HTTP server with LLM proxy endpoints
│   ├── loom-tools/          # Tool implementations
│   └── loom-cli/            # CLI binary
```

### Dependency Graph

```
                         ┌─────────────┐
                         │  loom-cli   │
                         └──────┬──────┘
                                │
              ┌─────────────────┼─────────────────┐
              │                 │                 │
              ▼                 ▼                 ▼
      ┌──────────────┐  ┌─────────────┐  ┌─────────────┐
      │loom-llm-proxy│  │ loom-tools  │  │    ...      │
      └──────┬───────┘  └──────┬──────┘  └─────────────┘
             │                 │
             ▼                 │
      ┌─────────────┐          │
      │ loom-server │          │
      └──────┬──────┘          │
             │                 │
             ▼                 │
    ┌────────────────┐         │
    │loom-llm-service│         │
    └───────┬────────┘         │
            │                  │
    ┌───────┴───────┐          │
    │               │          │
    ▼               ▼          │
┌────────────┐ ┌────────────┐  │
│loom-llm-   │ │loom-llm-   │  │
│anthropic   │ │openai      │  │
└─────┬──────┘ └─────┬──────┘  │
      │              │         │
      └──────┬───────┘         │
             ▼                 │
     ┌───────────────┐         │
     │loom-http-retry│         │
     └───────┬───────┘         │
             │                 │
             └─────────────────┘
                      │
                      ▼
              ┌─────────────┐
              │  loom-core  │
              └─────────────┘
```

## Server-Side LLM Proxy Architecture

Loom uses a server-side proxy architecture for all LLM interactions:

```
┌─────────────┐      HTTP       ┌─────────────┐     Provider API    ┌─────────────┐
│  loom-cli   │ ───────────────▶│ loom-server │ ──────────────────▶ │  Anthropic  │
│             │  /proxy/llm/*   │             │                     │   OpenAI    │
│ ProxyLlm-   │                 │  LlmService │                     │    etc.     │
│ Client      │ ◀─────────────  │             │ ◀────────────────── │             │
└─────────────┘   SSE stream    └─────────────┘    SSE stream       └─────────────┘
```

**Key properties:**

1. **API keys are ONLY stored server-side** - Clients never see or handle provider API keys
2. **Clients use `ProxyLlmClient`** - Implements `LlmClient` trait, calls `/proxy/llm/*` endpoints on the server
3. **Server uses `LlmService`** - Wraps provider clients (`AnthropicClient`, `OpenAIClient`), handles routing and provider selection
4. **Centralized provider management** - Adding new providers requires no client-side changes
5. **Security** - No secrets in client binaries, easier credential rotation, audit logging at proxy layer

### Request Flow

1. CLI creates `ProxyLlmClient` configured with server URL
2. `ProxyLlmClient.complete()` sends HTTP POST to `/proxy/llm/complete`
3. Server's `LlmService` routes to appropriate provider based on model
4. Provider client makes actual API call with server-stored credentials
5. Response streams back through server to client via SSE

## Design Principles

### Separation of Concerns

- **loom-core**: Defines interfaces (`LlmClient`, `ToolDefinition`) without implementations
- **loom-llm-proxy**: Client-side `ProxyLlmClient` that talks to server
- **loom-llm-service**: Server-side provider abstraction and routing
- **loom-llm-***: Provider-specific HTTP client implementations (server-only)
- **loom-tools**: Tool implementations that are LLM-agnostic
- **loom-cli**: Orchestration and user interaction

### Trait-Based Abstraction for LLM Providers

The `LlmClient` trait in `loom-core` defines the contract for all LLM providers:

```rust
#[async_trait]
pub trait LlmClient: Send + Sync {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError>;
    async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError>;
}
```

This enables:
- Runtime selection of providers
- Easy addition of new providers
- Testing with mock implementations

### Async-First Design with Tokio

All I/O operations are async:
- LLM API calls use streaming responses
- Tool executions are async for file I/O
- Retry logic uses `tokio::time::sleep`

### Structured Logging Throughout

Every crate uses `tracing` for structured, contextual logging:

```rust
#[instrument(skip(self, request), fields(model = %self.config.model))]
async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
    info!("Starting non-streaming completion request");
    // ...
}
```

## Dependency Flow

```
loom-core (bottom layer)
    ↑
loom-http-retry (utility layer)
    ↑
loom-llm-anthropic, loom-llm-openai (provider layer, server-only)
    ↑
loom-llm-service (server-side provider abstraction)
    ↑
loom-server (HTTP server with proxy endpoints)
    ↑
loom-llm-proxy (client-side LlmClient via HTTP)
    ↑
loom-tools (tool layer, depends only on loom-core)
    ↑
loom-cli (top layer, orchestrates everything)
```

**Key constraint**: Crates at lower layers never depend on higher layers. This ensures:
- Core types are reusable across all providers
- Provider implementations are isolated to server-side
- Clients interact only through the proxy abstraction
- The CLI can compose all components without provider dependencies

## Component Responsibilities

### loom-core

The foundation layer providing:

| Module | Responsibility |
|--------|---------------|
| `llm.rs` | `LlmClient` trait, `LlmRequest`, `LlmResponse`, `LlmStream`, `LlmEvent` |
| `tool.rs` | `ToolDefinition`, `ToolContext` |
| `message.rs` | `Message`, `Role`, `ToolCall` types |
| `state.rs` | `AgentState`, `AgentEvent`, `ToolExecutionStatus` enums |
| `agent.rs` | `Agent` struct and state machine logic |
| `config.rs` | `AgentConfig` with timeouts, retries, model settings |
| `error.rs` | Error types: `LlmError`, `ToolError`, `AgentError` |

### loom-http-retry

Resilient HTTP request handling:

- `RetryConfig` - Configurable retry parameters (max attempts, delays, jitter)
- `RetryableError` trait - Determines if an error should trigger retry
- `retry()` function - Generic retry wrapper with exponential backoff

### loom-llm-anthropic (server-only)

Anthropic Claude API implementation:

- `AnthropicClient` - Implements `LlmClient` for Claude models
- `AnthropicConfig` - API key, model, base URL configuration
- SSE stream parsing for streaming responses

### loom-llm-openai (server-only)

OpenAI API implementation:

- `OpenAIClient` - Implements `LlmClient` for GPT models
- `OpenAIConfig` - API key, model, base URL configuration

### loom-llm-service (server-only)

Server-side provider abstraction layer:

- `LlmService` - Wraps all provider clients, handles routing based on model
- Owns and manages API keys for all providers
- Provides unified interface for the server to call any provider

### loom-llm-proxy (client-side)

Client-side HTTP proxy client:

- `ProxyLlmClient` - Implements `LlmClient` trait via HTTP calls to server
- Sends requests to `/proxy/llm/complete` and `/proxy/llm/complete_streaming`
- Handles SSE stream parsing for streaming responses from server

### loom-server

HTTP server with LLM proxy endpoints:

- `/proxy/llm/complete` - Non-streaming completion endpoint
- `/proxy/llm/complete_streaming` - SSE streaming completion endpoint
- Uses `LlmService` to route requests to appropriate provider

### loom-tools

Tool implementations and registry:

- `Tool` trait - Interface for executable tools
- `ToolRegistry` - Registration and lookup of tools
- Built-in tools: `ReadFileTool`, `ListFilesTool`, `EditFileTool`

### loom-cli

Application entry point:

- CLI argument parsing with `clap`
- Creates `ProxyLlmClient` to communicate with server
- REPL loop for user interaction
- Tool execution orchestration

## Design Patterns Used

### State Machine Pattern (Agent)

The `Agent` uses an explicit state machine to manage conversation flow:

```rust
pub enum AgentState {
    WaitingForUserInput { conversation: ConversationContext },
    CallingLlm { conversation: ConversationContext, retries: u32 },
    ProcessingLlmResponse { conversation: ConversationContext, response: LlmResponse },
    ExecutingTools { conversation: ConversationContext, executions: Vec<ToolExecutionStatus> },
    Error { conversation: ConversationContext, error: AgentError, retries: u32, origin: ErrorOrigin },
    ShuttingDown,
}
```

State transitions are triggered by `AgentEvent`:

```rust
pub enum AgentEvent {
    UserInput(Message),
    LlmEvent(LlmEvent),
    ToolProgress(ToolProgressEvent),
    ToolCompleted { call_id: String, outcome: ToolExecutionOutcome },
    RetryTimeoutFired,
    ShutdownRequested,
}
```

The `handle_event()` method processes events and returns `AgentAction` for the caller:

```rust
pub enum AgentAction {
    SendLlmRequest(LlmRequest),
    ExecuteTools(Vec<ToolCall>),
    WaitForInput,
    DisplayMessage(String),
    DisplayError(String),
    Shutdown,
}
```

### Strategy Pattern (LlmClient Trait)

Different LLM providers implement the same interface, allowing runtime selection:

```rust
fn create_llm_client(provider: Provider, api_key: String) -> Arc<dyn LlmClient> {
    match provider {
        Provider::Anthropic => Arc::new(AnthropicClient::new(config)),
        Provider::OpenAi => Arc::new(OpenAIClient::new(config)),
    }
}
```

### Registry Pattern (ToolRegistry)

Tools are registered by name and looked up dynamically:

```rust
pub struct ToolRegistry {
    tools: HashMap<String, Box<dyn Tool>>,
}

impl ToolRegistry {
    pub fn register(&mut self, tool: Box<dyn Tool>) { ... }
    pub fn get(&self, name: &str) -> Option<&dyn Tool> { ... }
    pub fn definitions(&self) -> Vec<ToolDefinition> { ... }
}
```

### Builder Pattern (Configs)

Configuration structs use builder-style methods:

```rust
let config = AnthropicConfig::new(api_key)
    .with_model("claude-sonnet-4-20250514")
    .with_base_url("https://api.anthropic.com");

let request = LlmRequest::new("claude-sonnet-4-20250514")
    .with_messages(messages)
    .with_tools(tool_definitions)
    .with_max_tokens(4096);
```

### Discriminated Union / Sum Types

Rust enums encode all possible states with associated data:

**AgentState** - Conversation lifecycle states (see State Machine above)

**LlmEvent** - Streaming response events:
```rust
pub enum LlmEvent {
    TextDelta { content: String },
    ToolCallDelta { call_id: String, tool_name: String, arguments_fragment: String },
    Completed(LlmResponse),
    Error(LlmError),
}
```

**ToolExecutionStatus** - Tool lifecycle states:
```rust
pub enum ToolExecutionStatus {
    Pending { call_id: String, tool_name: String, requested_at: Instant },
    Running { call_id: String, tool_name: String, started_at: Instant, ... },
    Completed { call_id: String, tool_name: String, outcome: ToolExecutionOutcome, ... },
}
```

## Extension Points

### Adding a New LLM Provider

Provider clients are now **server-only**. Clients automatically get access to new providers via the proxy without any changes.

1. Create a new crate `loom-llm-{provider}`:

```rust
// crates/loom-llm-{provider}/src/client.rs
pub struct NewProviderClient { ... }

#[async_trait]
impl LlmClient for NewProviderClient {
    async fn complete(&self, request: LlmRequest) -> Result<LlmResponse, LlmError> {
        // Transform LlmRequest to provider's format
        // Make HTTP request
        // Transform response to LlmResponse
    }

    async fn complete_streaming(&self, request: LlmRequest) -> Result<LlmStream, LlmError> {
        // Similar, but return streaming response
    }
}
```

2. Add dependency in `loom-llm-service/Cargo.toml` (NOT in loom-cli)

3. Register the provider in `LlmService`:
```rust
// crates/loom-llm-service/src/service.rs
impl LlmService {
    pub fn new(config: LlmServiceConfig) -> Self {
        let mut providers = HashMap::new();
        providers.insert("anthropic", Arc::new(AnthropicClient::new(...)));
        providers.insert("openai", Arc::new(OpenAIClient::new(...)));
        providers.insert("new_provider", Arc::new(NewProviderClient::new(...))); // Add this
        // ...
    }
}
```

4. Configure API key in server configuration

**Note**: No client-side changes required. The `ProxyLlmClient` in loom-cli will automatically be able to use the new provider once the server is updated.

### Adding a New Tool

1. Implement the `Tool` trait in `loom-tools`:

```rust
// crates/loom-tools/src/my_tool.rs
pub struct MyTool { ... }

#[async_trait]
impl Tool for MyTool {
    fn name(&self) -> &str { "my_tool" }
    
    fn description(&self) -> &str { "Description for the LLM" }
    
    fn input_schema(&self) -> serde_json::Value {
        serde_json::json!({
            "type": "object",
            "properties": {
                "param": { "type": "string", "description": "Parameter description" }
            },
            "required": ["param"]
        })
    }
    
    async fn invoke(
        &self,
        args: serde_json::Value,
        ctx: &ToolContext,
    ) -> Result<serde_json::Value, ToolError> {
        // Parse args, execute logic, return result
    }
}
```

2. Register in `loom-cli`:
```rust
fn create_tool_registry() -> ToolRegistry {
    let mut registry = ToolRegistry::new();
    registry.register(Box::new(MyTool::new()));
    // ...
    registry
}
```

### Adding a New Agent State

1. Add variant to `AgentState` in `loom-core/src/state.rs`:
```rust
pub enum AgentState {
    // existing variants...
    NewState { conversation: ConversationContext, /* state data */ },
}
```

2. Add corresponding event if needed in `AgentEvent`

3. Update `Agent::handle_event()` with transition logic:
```rust
(AgentState::SomeState { .. }, AgentEvent::SomeEvent) => {
    self.state = AgentState::NewState { ... };
    AgentAction::SomeAction
}
```

4. Update `name()` method for logging
