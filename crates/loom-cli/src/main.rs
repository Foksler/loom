//! Loom CLI - Interactive AI coding assistant
//!
//! This binary provides a REPL interface for interacting with LLM-powered
//! coding agents. It supports multiple LLM providers and includes tools
//! for file system operations.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, ValueEnum};
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use loom_core::{LlmClient, LlmEvent, Message, ToolCall, ToolContext, ToolDefinition, ToolExecutionOutcome};
use loom_llm_anthropic::{AnthropicClient, AnthropicConfig};
use loom_llm_openai::{OpenAIClient, OpenAIConfig};
use loom_tools::{EditFileTool, ListFilesTool, ReadFileTool, ToolRegistry};

/// LLM provider selection
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Provider {
    #[default]
    Anthropic,
    OpenAi,
}

/// Log level selection
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}

impl From<LogLevel> for tracing::Level {
    fn from(level: LogLevel) -> Self {
        match level {
            LogLevel::Trace => tracing::Level::TRACE,
            LogLevel::Debug => tracing::Level::DEBUG,
            LogLevel::Info => tracing::Level::INFO,
            LogLevel::Warn => tracing::Level::WARN,
            LogLevel::Error => tracing::Level::ERROR,
        }
    }
}

/// Loom - AI-powered coding assistant
#[derive(Parser, Debug)]
#[command(name = "loom", version, about, long_about = None)]
struct Args {
    /// LLM provider to use
    #[arg(short, long, default_value = "anthropic")]
    provider: Provider,

    /// Model name (uses provider default if not specified)
    #[arg(short, long)]
    model: Option<String>,

    /// API key for the selected provider.
    /// Can also be set via ANTHROPIC_API_KEY or OPENAI_API_KEY environment variables.
    #[arg(long, env = "ANTHROPIC_API_KEY")]
    api_key: Option<String>,

    /// Workspace directory for file operations
    #[arg(short, long, default_value = ".")]
    workspace: PathBuf,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: LogLevel,

    /// Output logs as JSON
    #[arg(long, default_value = "false")]
    json_logs: bool,
}

impl Args {
    /// Resolve the API key based on provider, checking provider-specific env vars
    fn resolve_api_key(&self) -> Result<String> {
        if let Some(ref key) = self.api_key {
            return Ok(key.clone());
        }

        let env_var = match self.provider {
            Provider::Anthropic => "ANTHROPIC_API_KEY",
            Provider::OpenAi => "OPENAI_API_KEY",
        };

        std::env::var(env_var)
            .with_context(|| format!("API key not provided. Set --api-key or {}", env_var))
    }
}

/// Initialize the tracing subscriber based on configuration
fn init_tracing(log_level: LogLevel, json_logs: bool) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("loom={}", tracing::Level::from(log_level))));

    if json_logs {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer().json())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(filter)
            .with(fmt::layer())
            .init();
    }
}

/// Create an LLM client based on the selected provider
#[instrument(skip(api_key))]
fn create_llm_client(
    provider: Provider,
    api_key: String,
    model: Option<String>,
) -> Result<Arc<dyn LlmClient>> {
    info!(
        provider = ?provider,
        model = ?model,
        "creating LLM client"
    );

    match provider {
        Provider::Anthropic => {
            let mut config = AnthropicConfig::new(api_key);
            if let Some(m) = model {
                config = config.with_model(m);
            }
            let client = AnthropicClient::new(config)
                .context("failed to create Anthropic client")?;
            Ok(Arc::new(client))
        }
        Provider::OpenAi => {
            let mut config = OpenAIConfig::new(api_key);
            if let Some(m) = model {
                config = config.with_model(m);
            }
            let client = OpenAIClient::new(config)
                .context("failed to create OpenAI client")?;
            Ok(Arc::new(client))
        }
    }
}

/// Create the tool registry with available tools
#[instrument]
fn create_tool_registry() -> ToolRegistry {
    info!("creating tool registry");

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(ReadFileTool::new()));
    registry.register(Box::new(ListFilesTool::new()));
    registry.register(Box::new(EditFileTool::new()));

    let definitions = registry.definitions();
    debug!(
        tool_count = definitions.len(),
        "tool registry initialized"
    );

    registry
}

/// Get tool definitions from the registry
fn get_tool_definitions(registry: &ToolRegistry) -> Vec<ToolDefinition> {
    registry.definitions()
}

/// Execute a tool call using the registry
#[instrument(skip(registry, ctx))]
async fn execute_tool(
    registry: &ToolRegistry,
    tool_call: &ToolCall,
    ctx: &ToolContext,
) -> ToolExecutionOutcome {
    debug!(
        tool_id = %tool_call.id,
        tool_name = %tool_call.tool_name,
        "executing tool"
    );

    match registry.get(&tool_call.tool_name) {
        Some(tool) => {
            match tool.invoke(tool_call.arguments_json.clone(), ctx).await {
                Ok(output) => {
                    debug!(
                        tool_id = %tool_call.id,
                        "tool execution succeeded"
                    );
                    ToolExecutionOutcome::Success {
                        call_id: tool_call.id.clone(),
                        output,
                    }
                }
                Err(e) => {
                    warn!(
                        tool_id = %tool_call.id,
                        error = %e,
                        "tool execution failed"
                    );
                    ToolExecutionOutcome::Error {
                        call_id: tool_call.id.clone(),
                        error: e,
                    }
                }
            }
        }
        None => {
            warn!(
                tool_id = %tool_call.id,
                tool_name = %tool_call.tool_name,
                "tool not found"
            );
            ToolExecutionOutcome::Error {
                call_id: tool_call.id.clone(),
                error: loom_core::ToolError::NotFound(tool_call.tool_name.clone()),
            }
        }
    }
}

/// Run the REPL loop
#[instrument(skip(llm_client, tool_registry, tool_ctx))]
async fn run_repl(
    llm_client: &dyn LlmClient,
    tool_registry: &ToolRegistry,
    tool_definitions: &[ToolDefinition],
    tool_ctx: &ToolContext,
) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("Welcome to Loom - AI-powered coding assistant");
    println!("Type your message and press Enter. Use Ctrl+C to exit.\n");

    let mut messages: Vec<Message> = Vec::new();

    loop {
        print!("> ");
        stdout.flush()?;

        let mut input = String::new();
        let bytes_read = stdin.lock().read_line(&mut input)?;

        if bytes_read == 0 {
            info!("EOF received, shutting down");
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        debug!(input_length = input.len(), "received user input");

        let user_message = Message::user(input);
        messages.push(user_message);

        let request = loom_core::LlmRequest::new("default")
            .with_messages(messages.clone())
            .with_tools(tool_definitions.to_vec());

        match llm_client.complete_streaming(request).await {
            Ok(mut stream) => {
                let mut assistant_content = String::new();
                let mut tool_calls: Vec<ToolCall> = Vec::new();

                while let Some(event) = stream.next().await {
                    match event {
                        LlmEvent::TextDelta { content } => {
                            print!("{}", content);
                            let _ = io::stdout().flush();
                            assistant_content.push_str(&content);
                        }
                        LlmEvent::ToolCallDelta { call_id, tool_name, arguments_fragment } => {
                            debug!(
                                call_id = %call_id,
                                tool_name = %tool_name,
                                fragment_len = arguments_fragment.len(),
                                "tool call delta"
                            );
                        }
                        LlmEvent::Completed(response) => {
                            info!(
                                finish_reason = ?response.finish_reason,
                                "LLM response complete"
                            );
                            println!();
                            tool_calls = response.tool_calls;
                            if !response.message.content.is_empty() {
                                assistant_content = response.message.content.clone();
                            }
                        }
                        LlmEvent::Error(e) => {
                            error!(error = ?e, "LLM stream error");
                        }
                    }
                }

                messages.push(Message::assistant(&assistant_content));

                for tool_call in tool_calls {
                    info!(
                        tool_name = %tool_call.tool_name,
                        tool_id = %tool_call.id,
                        "executing tool call"
                    );

                    let outcome = execute_tool(tool_registry, &tool_call, tool_ctx).await;

                    let (tool_result, is_error) = match &outcome {
                        ToolExecutionOutcome::Success { output, .. } => {
                            (output.to_string(), false)
                        }
                        ToolExecutionOutcome::Error { error, .. } => {
                            (format!("Error: {}", error), true)
                        }
                    };

                    if is_error {
                        warn!(tool_id = %tool_call.id, result = %tool_result, "tool returned error");
                    } else {
                        debug!(tool_id = %tool_call.id, "tool completed successfully");
                    }

                    messages.push(Message::tool(&tool_call.id, &tool_call.tool_name, &tool_result));
                }
            }
            Err(e) => {
                error!(error = %e, "failed to start LLM request");
                eprintln!("Error: {}", e);
            }
        }
    }

    Ok(())
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    init_tracing(args.log_level, args.json_logs);

    info!(
        provider = ?args.provider,
        workspace = %args.workspace.display(),
        "starting loom"
    );

    let api_key = args.resolve_api_key()?;

    let workspace = args
        .workspace
        .canonicalize()
        .with_context(|| format!("invalid workspace path: {}", args.workspace.display()))?;

    let llm_client = create_llm_client(args.provider, api_key, args.model)?;
    let tool_registry = create_tool_registry();
    let tool_definitions = get_tool_definitions(&tool_registry);
    let tool_ctx = ToolContext::new(&workspace);

    info!(
        tool_count = tool_definitions.len(),
        workspace = %workspace.display(),
        "initialized"
    );

    ctrlc_handler()?;

    run_repl(llm_client.as_ref(), &tool_registry, &tool_definitions, &tool_ctx).await
}

/// Set up Ctrl+C handler for graceful shutdown
fn ctrlc_handler() -> Result<()> {
    ctrlc::set_handler(|| {
        info!("received Ctrl+C, shutting down");
        println!("\nInterrupted. Goodbye!");
        std::process::exit(0);
    })
    .context("failed to set Ctrl+C handler")?;

    Ok(())
}
