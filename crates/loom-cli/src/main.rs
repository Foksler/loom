//! Loom CLI - Interactive AI coding assistant
//!
//! This binary provides a REPL interface for interacting with LLM-powered
//! coding agents. It supports multiple LLM providers and includes tools
//! for file system operations.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::Parser;
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use loom_config::{
    load_config_with_cli,
    runtime::{LogFormat, LogLevel, ProviderConfig},
    sources::CliOverrides,
};
use loom_core::{LlmClient, LlmEvent, Message, ToolCall, ToolContext, ToolDefinition, ToolExecutionOutcome};
use loom_llm_anthropic::{AnthropicClient, AnthropicConfig};
use loom_llm_openai::{OpenAIClient, OpenAIConfig};
use loom_tools::{EditFileTool, ListFilesTool, ReadFileTool, ToolRegistry};

/// Loom - AI-powered coding assistant
#[derive(Parser, Debug)]
#[command(name = "loom", version, about, long_about = None)]
struct Args {
    /// Path to custom configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// LLM provider to use (overrides config)
    #[arg(short, long)]
    provider: Option<String>,

    /// Model name (overrides config)
    #[arg(short, long)]
    model: Option<String>,

    /// Workspace directory for file operations
    #[arg(short, long)]
    workspace: Option<PathBuf>,

    /// Log level (overrides config)
    #[arg(short, long)]
    log_level: Option<String>,

    /// Output logs as JSON (overrides config)
    #[arg(long)]
    json_logs: bool,
}

impl From<&Args> for CliOverrides {
    fn from(args: &Args) -> Self {
        Self {
            provider: args.provider.clone(),
            model: args.model.clone(),
            workspace: args.workspace.clone(),
            log_level: args.log_level.clone(),
            log_format: if args.json_logs {
                Some("json".to_string())
            } else {
                None
            },
            config_file: args.config.clone(),
        }
    }
}

fn log_level_to_tracing(level: LogLevel) -> tracing::Level {
    match level {
        LogLevel::Trace => tracing::Level::TRACE,
        LogLevel::Debug => tracing::Level::DEBUG,
        LogLevel::Info => tracing::Level::INFO,
        LogLevel::Warn => tracing::Level::WARN,
        LogLevel::Error => tracing::Level::ERROR,
    }
}

fn init_tracing(logging: &loom_config::runtime::LoggingConfig) {
    let filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(format!("loom={}", log_level_to_tracing(logging.level))));

    match logging.format {
        LogFormat::Json => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().json())
                .init();
        }
        LogFormat::Compact => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer().compact())
                .init();
        }
        LogFormat::Pretty => {
            tracing_subscriber::registry()
                .with(filter)
                .with(fmt::layer())
                .init();
        }
    }
}

#[instrument(skip(provider_config))]
fn create_llm_client(
    provider_name: &str,
    provider_config: &ProviderConfig,
    model_override: Option<&str>,
) -> Result<Arc<dyn LlmClient>> {
    info!(
        provider = %provider_name,
        model_override = ?model_override,
        "creating LLM client"
    );

    match provider_config {
        ProviderConfig::Anthropic(cfg) => {
            let model = model_override
                .map(String::from)
                .unwrap_or_else(|| cfg.default_model.clone());
            let config = AnthropicConfig::new(&cfg.api_key).with_model(model);
            let client = AnthropicClient::new(config)
                .context("failed to create Anthropic client")?;
            Ok(Arc::new(client))
        }
        ProviderConfig::OpenAi(cfg) => {
            let model = model_override
                .map(String::from)
                .unwrap_or_else(|| cfg.default_model.clone());
            let config = OpenAIConfig::new(&cfg.api_key).with_model(model);
            let client = OpenAIClient::new(config)
                .context("failed to create OpenAI client")?;
            Ok(Arc::new(client))
        }
        ProviderConfig::Ollama(_) => {
            anyhow::bail!("Ollama provider not yet implemented")
        }
        ProviderConfig::Custom(_) => {
            anyhow::bail!("Custom provider not yet implemented")
        }
    }
}

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

fn get_tool_definitions(registry: &ToolRegistry) -> Vec<ToolDefinition> {
    registry.definitions()
}

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

    let cli_overrides = CliOverrides::from(&args);
    let config = load_config_with_cli(cli_overrides)
        .context("failed to load configuration")?;

    init_tracing(&config.logging);

    info!(
        provider = %config.global.default_provider,
        "starting loom"
    );

    let provider_config = config
        .providers
        .get(&config.global.default_provider)
        .with_context(|| {
            format!(
                "provider '{}' not configured. Available: {:?}",
                config.global.default_provider,
                config.providers.keys().collect::<Vec<_>>()
            )
        })?;

    let workspace = config
        .global
        .workspace_root
        .clone()
        .or_else(|| config.tools.workspace.root.clone())
        .unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()
        .context("invalid workspace path")?;

    let llm_client = create_llm_client(
        &config.global.default_provider,
        provider_config,
        args.model.as_deref(),
    )?;

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

fn ctrlc_handler() -> Result<()> {
    ctrlc::set_handler(|| {
        info!("received Ctrl+C, shutting down");
        println!("\nInterrupted. Goodbye!");
        std::process::exit(0);
    })
    .context("failed to set Ctrl+C handler")?;

    Ok(())
}
