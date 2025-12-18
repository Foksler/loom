//! Loom CLI - Interactive AI coding assistant
//!
//! This binary provides a REPL interface for interacting with LLM-powered
//! coding agents. It supports multiple LLM providers and includes tools
//! for file system operations.

use std::io::{self, BufRead, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};
use tracing::{debug, error, info, instrument, warn};
use tracing_subscriber::{fmt, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use loom_config::{
    load_config_with_cli,
    runtime::{LogFormat, LogLevel},
    sources::CliOverrides,
};
use loom_core::{LlmClient, LlmEvent, Message, ToolCall, ToolContext, ToolDefinition, ToolExecutionOutcome};
use loom_llm_proxy::{LlmProvider, ProxyLlmClient};
use loom_thread::{
    Thread, ThreadId, ThreadStore, LocalThreadStore, SyncingThreadStore,
    ThreadSyncClient, MessageSnapshot, AgentStateKind, AgentStateSnapshot,
    MessageRole, ToolCallSnapshot, LoomVersionHeaders, ThreadVisibility,
};
use loom_git::detect_repo_status;

#[derive(clap::ValueEnum, Clone, Debug)]
enum ShareVisibilityArg {
    Organization,
    Private,
    Public,
}

impl From<ShareVisibilityArg> for ThreadVisibility {
    fn from(v: ShareVisibilityArg) -> Self {
        match v {
            ShareVisibilityArg::Organization => ThreadVisibility::Organization,
            ShareVisibilityArg::Private => ThreadVisibility::Private,
            ShareVisibilityArg::Public => ThreadVisibility::Public,
        }
    }
}
use loom_tools::{EditFileTool, ListFilesTool, ReadFileTool, ToolRegistry, WebSearchTool};
use url::Url;

mod version;

/// Loom - AI-powered coding assistant
#[derive(Parser, Debug)]
#[command(name = "loom", version, about, long_about = None)]
struct Args {
    /// Path to custom configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Workspace directory for file operations
    #[arg(short, long)]
    workspace: Option<PathBuf>,

    /// Log level (overrides config)
    #[arg(short, long)]
    log_level: Option<String>,

    /// Output logs as JSON (overrides config)
    #[arg(long)]
    json_logs: bool,

    /// Loom server URL for LLM proxy
    #[arg(long, env = "LOOM_SERVER_URL", default_value = "http://localhost:8080")]
    server_url: String,

    /// LLM provider to use (anthropic or openai)
    #[arg(short, long, env = "LOOM_LLM_PROVIDER", default_value = "anthropic")]
    provider: String,

    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Subcommand, Debug)]
enum Command {
    /// Authenticate with Loom services
    Login,
    /// Log out from Loom services
    Logout,
    /// List local threads
    List,
    /// Resume an existing thread
    Resume {
        /// Thread ID to resume (uses most recent if not specified)
        thread_id: Option<String>,
    },
    /// Start a new private (local-only) session that never syncs
    Private,
    /// Change server-side visibility of a synced thread
    Share {
        /// Thread ID to share (uses most recent if not specified)
        thread_id: Option<String>,
        /// Desired visibility: organization, private, support, or public
        #[arg(long, value_enum, conflicts_with = "support")]
        visibility: Option<ShareVisibilityArg>,
        /// Share thread with support team (shortcut for --visibility support)
        #[arg(long)]
        support: bool,
    },
    /// Search threads by content, git metadata, or commit SHA
    Search {
        /// Search query (text, branch name, repo URL, or commit SHA prefix)
        query: String,
        /// Maximum number of results
        #[arg(short, long, default_value = "20")]
        limit: usize,
        /// Output raw JSON
        #[arg(long)]
        json: bool,
    },
    /// Show version and build information
    Version,
    /// Update Loom to the latest version from the server
    Update,
}

impl From<&Args> for CliOverrides {
    fn from(args: &Args) -> Self {
        Self {
            provider: None,
            model: None,
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

fn create_llm_client(server_url: &str, provider: &str) -> Result<Arc<dyn LlmClient>> {
    let llm_provider = match provider.to_lowercase().as_str() {
        "anthropic" => LlmProvider::Anthropic,
        "openai" => LlmProvider::OpenAi,
        other => anyhow::bail!("Unknown LLM provider: {}. Use 'anthropic' or 'openai'", other),
    };

    info!(server_url = %server_url, provider = %provider, "creating proxy LLM client");
    let client = ProxyLlmClient::new(server_url, llm_provider);
    Ok(Arc::new(client))
}

#[instrument]
fn create_tool_registry() -> ToolRegistry {
    info!("creating tool registry");

    let mut registry = ToolRegistry::new();
    registry.register(Box::new(ReadFileTool::new()));
    registry.register(Box::new(ListFilesTool::new()));
    registry.register(Box::new(EditFileTool::new()));
    registry.register(Box::new(WebSearchTool::default()));

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

#[instrument(skip(llm_client, tool_registry, tool_ctx, thread, thread_store, shutdown_flag, workspace))]
async fn run_repl(
    llm_client: &dyn LlmClient,
    tool_registry: &ToolRegistry,
    tool_definitions: &[ToolDefinition],
    tool_ctx: &ToolContext,
    thread: &mut Thread,
    thread_store: &dyn ThreadStore,
    shutdown_flag: Arc<AtomicBool>,
    workspace: &std::path::Path,
) -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    println!("Welcome to Loom - AI-powered coding assistant");
    println!("Thread: {}", thread.id);
    println!("Type your message and press Enter. Use Ctrl+C to exit.\n");

    let mut messages: Vec<Message> = Vec::new();

    loop {
        // Check if shutdown was requested
        if shutdown_flag.load(Ordering::Relaxed) {
            info!("shutdown requested, saving thread");
            snapshot_git_state(thread, workspace);
            thread.touch();
            if let Err(e) = thread_store.save(thread).await {
                warn!(error = %e, "failed to save thread on shutdown");
            }
            println!("\nInterrupted. Thread saved. Goodbye!");
            break;
        }

        print!("> ");
        stdout.flush()?;

        let mut input = String::new();
        let bytes_read = stdin.lock().read_line(&mut input)?;

        if bytes_read == 0 {
            info!("EOF received, shutting down");
            snapshot_git_state(thread, workspace);
            thread.touch();
            if let Err(e) = thread_store.save(thread).await {
                warn!(error = %e, "failed to save thread on exit");
            }
            break;
        }
        
        // Check shutdown again after potentially blocking read
        if shutdown_flag.load(Ordering::Relaxed) {
            info!("shutdown requested after input, saving thread");
            snapshot_git_state(thread, workspace);
            thread.touch();
            if let Err(e) = thread_store.save(thread).await {
                warn!(error = %e, "failed to save thread on shutdown");
            }
            println!("\nInterrupted. Thread saved. Goodbye!");
            break;
        }

        let input = input.trim();
        if input.is_empty() {
            continue;
        }

        debug!(input_length = input.len(), "received user input");

        let user_message = Message::user(input);
        messages.push(user_message.clone());

        thread.conversation.messages.push(MessageSnapshot::from(&user_message));

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

                thread.conversation.messages.push(MessageSnapshot {
                    role: MessageRole::Assistant,
                    content: assistant_content.clone(),
                    tool_call_id: None,
                    tool_name: None,
                    tool_calls: if tool_calls.is_empty() { None } else {
                        Some(tool_calls.iter().map(|tc| ToolCallSnapshot {
                            id: tc.id.clone(),
                            tool_name: tc.tool_name.clone(),
                            arguments_json: tc.arguments_json.clone(),
                        }).collect())
                    },
                });

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

                    thread.conversation.messages.push(MessageSnapshot {
                        role: MessageRole::Tool,
                        content: tool_result.clone(),
                        tool_call_id: Some(tool_call.id.clone()),
                        tool_name: Some(tool_call.tool_name.clone()),
                        tool_calls: None,
                    });
                }

                thread.agent_state = AgentStateSnapshot {
                    kind: AgentStateKind::WaitingForUserInput,
                    retries: 0,
                    last_error: None,
                    pending_tool_calls: Vec::new(),
                };
                snapshot_git_state(thread, workspace);
                thread.touch();

                if let Err(e) = thread_store.save(thread).await {
                    warn!(error = %e, "failed to save thread");
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

async fn start_repl_session(
    config: &loom_config::LoomConfig,
    args: &Args,
    thread_store: Arc<dyn ThreadStore>,
    mut thread: Thread,
) -> Result<()> {
    let workspace = config
        .global
        .workspace_root
        .clone()
        .or_else(|| config.tools.workspace.root.clone())
        .unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()
        .context("invalid workspace path")?;

    let llm_client = create_llm_client(&args.server_url, &args.provider)?;

    let tool_registry = create_tool_registry();
    let tool_definitions = get_tool_definitions(&tool_registry);
    let tool_ctx = ToolContext::new(&workspace);

    info!(
        tool_count = tool_definitions.len(),
        workspace = %workspace.display(),
        "initialized"
    );

    let shutdown_flag = Arc::new(AtomicBool::new(false));
    setup_ctrlc_handler(shutdown_flag.clone())?;

    run_repl(
        llm_client.as_ref(),
        &tool_registry,
        &tool_definitions,
        &tool_ctx,
        &mut thread,
        thread_store.as_ref(),
        shutdown_flag,
        &workspace,
    ).await
}

fn get_update_base_url() -> Result<Url> {
    if let Ok(raw) = std::env::var("LOOM_UPDATE_BASE_URL") {
        return Url::parse(&raw).context("invalid LOOM_UPDATE_BASE_URL");
    }
    if let Ok(sync_url) = std::env::var("LOOM_THREAD_SYNC_URL") {
        let mut base = Url::parse(&sync_url).context("invalid LOOM_THREAD_SYNC_URL")?;
        base.set_path("");
        return Ok(base);
    }
    anyhow::bail!("LOOM_UPDATE_BASE_URL or LOOM_THREAD_SYNC_URL must be set for updates")
}

async fn run_update() -> Result<()> {
    let build_info = version::build_info();
    let base_url = get_update_base_url()?;
    
    let bin_url = base_url
        .join(&format!("bin/{}", build_info.platform))
        .context("failed to construct update URL")?;
    
    println!("Current version: {}", build_info.version);
    println!("Platform:        {}", build_info.platform);
    println!("Checking for updates from {}...", bin_url);
    
    let current_exe = std::env::current_exe()
        .context("failed to get current executable path")?;
    
    let http_client = reqwest::Client::new();
    let response = http_client
        .get(bin_url.clone())
        .header("X-Loom-Version", build_info.version)
        .header("X-Loom-Git-Sha", build_info.git_sha)
        .header("X-Loom-Platform", build_info.platform)
        .send()
        .await
        .context("failed to download update")?;
    
    if !response.status().is_success() {
        anyhow::bail!(
            "Update server returned error: {} - {}",
            response.status(),
            response.text().await.unwrap_or_default()
        );
    }
    
    let bytes = response.bytes().await.context("failed to read update binary")?;
    
    if bytes.is_empty() {
        anyhow::bail!("Downloaded binary is empty");
    }
    
    println!("Downloaded {} bytes", bytes.len());
    
    let tmp_path = current_exe.with_extension("new");
    tokio::fs::write(&tmp_path, &bytes)
        .await
        .context("failed to write temporary binary")?;
    
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut perms = std::fs::metadata(&tmp_path)?.permissions();
        perms.set_mode(0o755);
        std::fs::set_permissions(&tmp_path, perms)?;
    }
    
    let backup_path = current_exe.with_extension("old");
    if backup_path.exists() {
        std::fs::remove_file(&backup_path).ok();
    }
    
    std::fs::rename(&current_exe, &backup_path)
        .context("failed to backup current binary")?;
    std::fs::rename(&tmp_path, &current_exe)
        .context("failed to install new binary")?;
    
    println!("Update complete! Please restart loom.");
    
    Ok(())
}

fn snapshot_git_state(thread: &mut Thread, workspace_path: &std::path::Path) {
    match detect_repo_status(workspace_path) {
        Ok(Some(status)) => {
            if thread.git_remote_url.is_none() {
                thread.git_remote_url = status.remote_slug.clone();
            }

            if thread.git_initial_branch.is_none() {
                thread.git_initial_branch = status.branch.clone();
            }

            thread.git_branch = status.branch;

            if let Some(ref head) = status.head {
                let sha = head.sha.clone();

                if thread.git_initial_commit_sha.is_none() {
                    thread.git_initial_commit_sha = Some(sha.clone());
                }

                thread.git_current_commit_sha = Some(sha.clone());

                if !thread.git_commits.contains(&sha) {
                    thread.git_commits.push(sha);
                }
            }

            if thread.git_start_dirty.is_none() {
                thread.git_start_dirty = status.is_dirty;
            }
            thread.git_end_dirty = status.is_dirty;

            debug!(
                git_branch = ?thread.git_branch,
                git_remote_url = ?thread.git_remote_url,
                git_current_commit_sha = ?thread.git_current_commit_sha,
                git_is_dirty = ?thread.git_end_dirty,
                "snapshot git state"
            );
        }
        Ok(None) => {
            debug!("not a git repository or git unavailable");
        }
        Err(e) => {
            debug!(error = %e, "failed to detect git repository");
        }
    }
}

async fn run_search(
    query: &str,
    limit: usize,
    json_output: bool,
    _thread_store: &dyn ThreadStore,
) -> Result<()> {
    let query = query.trim();
    if query.is_empty() {
        anyhow::bail!("Search query cannot be empty");
    }
    
    // Try server search first if sync is enabled
    if let Ok(sync_url) = std::env::var("LOOM_THREAD_SYNC_URL") {
        match search_server(&sync_url, query, limit).await {
            Ok(results) => {
                if json_output {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                } else {
                    print_search_results(&results, query);
                }
                return Ok(());
            }
            Err(e) => {
                debug!(error = %e, "Server search failed, falling back to local");
            }
        }
    }
    
    // Fall back to local search
    let local_store = LocalThreadStore::from_xdg()?;
    let results = local_store.search(query, limit).await?;
    
    if json_output {
        println!("{}", serde_json::to_string_pretty(&results)?);
    } else {
        print_local_search_results(&results, query);
    }
    
    Ok(())
}

async fn search_server(base_url: &str, query: &str, limit: usize) -> Result<Vec<serde_json::Value>> {
    let client = reqwest::Client::new();
    let url = format!("{}/v1/threads/search", base_url.trim_end_matches('/'));
    
    let response = client
        .get(&url)
        .query(&[("q", query), ("limit", &limit.to_string())])
        .timeout(std::time::Duration::from_secs(10))
        .send()
        .await?;
    
    if !response.status().is_success() {
        anyhow::bail!("Server returned {}", response.status());
    }
    
    let body: serde_json::Value = response.json().await?;
    let hits = body.get("hits")
        .and_then(|h| h.as_array())
        .cloned()
        .unwrap_or_default();
    
    Ok(hits)
}

fn print_search_results(results: &[serde_json::Value], query: &str) {
    if results.is_empty() {
        println!("No results found for \"{}\"", query);
        return;
    }
    
    println!("Results for \"{}\" ({} hits):\n", query, results.len());
    
    for (i, hit) in results.iter().enumerate() {
        let summary = hit.get("summary").unwrap_or(hit);
        let id = summary.get("id").and_then(|v| v.as_str()).unwrap_or("?");
        let title = summary.get("title").and_then(|v| v.as_str()).unwrap_or("(untitled)");
        let branch = summary.get("git_branch").and_then(|v| v.as_str()).unwrap_or("-");
        let remote = summary.get("git_remote_url").and_then(|v| v.as_str()).unwrap_or("-");
        let score = hit.get("score").and_then(|v| v.as_f64());
        
        println!("{}) {}", i + 1, id);
        if remote != "-" {
            println!("   [{}] {}", remote, branch);
        }
        println!("   \"{}\"", title);
        if let Some(s) = score {
            if s != 0.0 {
                println!("   score: {:.3}", s);
            }
        }
        println!();
    }
}

fn print_local_search_results(results: &[loom_thread::ThreadSummary], query: &str) {
    if results.is_empty() {
        println!("No results found for \"{}\" (local search)", query);
        return;
    }
    
    println!("Results for \"{}\" ({} hits, local search):\n", query, results.len());
    
    for (i, summary) in results.iter().enumerate() {
        let title = summary.title.as_deref().unwrap_or("(untitled)");
        let branch = summary.git_branch.as_deref().unwrap_or("-");
        let remote = summary.git_remote_url.as_deref().unwrap_or("-");
        
        println!("{}) {}", i + 1, summary.id);
        if remote != "-" {
            println!("   [{}] {}", remote, branch);
        }
        println!("   \"{}\"", title);
        println!();
    }
}

fn create_new_thread(config: &loom_config::LoomConfig, args: &Args) -> Result<Thread> {
    let workspace = config
        .global
        .workspace_root
        .clone()
        .or_else(|| config.tools.workspace.root.clone())
        .unwrap_or_else(|| PathBuf::from("."))
        .canonicalize()
        .context("invalid workspace path")?;

    let mut thread = Thread::new();
    thread.workspace_root = Some(workspace.display().to_string());
    thread.cwd = Some(std::env::current_dir()?.display().to_string());
    thread.loom_version = Some(env!("CARGO_PKG_VERSION").to_string());
    thread.provider = Some(args.provider.clone());
    thread.model = None;

    snapshot_git_state(&mut thread, &workspace);

    info!(thread_id = %thread.id, provider = %args.provider, "created new thread");
    Ok(thread)
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

    let thread_store: Arc<dyn ThreadStore> = {
        let local_store = LocalThreadStore::from_xdg()
            .context("failed to create local thread store")?;

        if let Ok(sync_url) = std::env::var("LOOM_THREAD_SYNC_URL") {
            let base_url = Url::parse(&sync_url)
                .context("invalid LOOM_THREAD_SYNC_URL")?;
            let http_client = reqwest::Client::new();
            
            let build_info = version::build_info();
            let version_headers = LoomVersionHeaders {
                version: build_info.version.to_string(),
                git_sha: build_info.git_sha.to_string(),
                build_timestamp: build_info.build_timestamp.to_string(),
                platform: build_info.platform.to_string(),
            };
            
            let sync_client = ThreadSyncClient::new(base_url, http_client)
                .with_version_headers(version_headers);
            Arc::new(SyncingThreadStore::with_sync(local_store, sync_client))
        } else {
            Arc::new(SyncingThreadStore::local_only(local_store))
        }
    };

    match &args.command {
        Some(Command::Version) => {
            println!("{}", version::format_version_info());
            Ok(())
        }
        Some(Command::Update) => {
            run_update().await
        }
        Some(Command::Login) => {
            println!("loom login: not implemented yet");
            Ok(())
        }
        Some(Command::Logout) => {
            println!("loom logout: not implemented yet");
            Ok(())
        }
        Some(Command::List) => {
            let threads = thread_store.list(100).await
                .context("failed to list threads")?;

            if threads.is_empty() {
                println!("No threads found.");
            } else {
                println!("{:<42} {:<30} {:>6} {:<20}", "ID", "TITLE", "MSGS", "LAST ACTIVITY");
                println!("{}", "-".repeat(100));
                for summary in threads {
                    let title = summary.title.as_deref().unwrap_or("(untitled)");
                    let title_display = if title.len() > 28 {
                        format!("{}...", &title[..25])
                    } else {
                        title.to_string()
                    };
                    println!(
                        "{:<42} {:<30} {:>6} {:<20}",
                        summary.id,
                        title_display,
                        summary.message_count,
                        summary.last_activity_at
                    );
                }
            }
            Ok(())
        }
        Some(Command::Resume { thread_id }) => {
            let thread = match thread_id {
                Some(id) => {
                    let tid = ThreadId::from_string(id.clone());
                    thread_store.load(&tid).await
                        .context("failed to load thread")?
                        .with_context(|| format!("thread '{}' not found", id))?
                }
                None => {
                    let threads = thread_store.list(1).await
                        .context("failed to list threads")?;
                    let summary = threads.into_iter().next()
                        .context("no threads found to resume")?;
                    thread_store.load(&summary.id).await
                        .context("failed to load thread")?
                        .context("thread not found")?
                }
            };
            info!(thread_id = %thread.id, "resuming thread");
            start_repl_session(&config, &args, thread_store, thread).await
        }
        Some(Command::Private) => {
            let mut thread = create_new_thread(&config, &args)?;
            thread.is_private = true;
            thread.visibility = ThreadVisibility::Private;
            info!(thread_id = %thread.id, "created new private (local-only) thread");
            println!("Starting private session (local-only, never synced to server).");
            println!("Thread: {}", thread.id);
            start_repl_session(&config, &args, thread_store, thread).await
        }
        Some(Command::Search { query, limit, json }) => {
            run_search(&query, *limit, *json, thread_store.as_ref()).await
        }
        Some(Command::Share { thread_id, visibility, support }) => {
            let thread = match thread_id {
                Some(id) => {
                    let tid = ThreadId::from_string(id.clone());
                    thread_store.load(&tid).await
                        .context("failed to load thread")?
                        .with_context(|| format!("thread '{}' not found", id))?
                }
                None => {
                    let threads = thread_store.list(1).await
                        .context("failed to list threads")?;
                    let summary = threads.into_iter().next()
                        .context("no threads found to share")?;
                    thread_store.load(&summary.id).await
                        .context("failed to load thread")?
                        .context("thread not found")?
                }
            };

            if thread.is_private {
                anyhow::bail!(
                    "Thread {} is a local-only private session and cannot be shared. \
                     Start a normal session if you want to sync to the server.",
                    thread.id
                );
            }

            let mut updated = thread.clone();

            if *support {
                updated.is_shared_with_support = true;
                updated.touch();

                info!(
                    thread_id = %updated.id,
                    "sharing thread with support"
                );

                thread_store.save(&updated).await
                    .context("failed to save thread")?;

                println!(
                    "Thread {} has been shared with support.",
                    updated.id
                );
            } else if let Some(v) = visibility {
                updated.visibility = ThreadVisibility::from(v.clone());
                updated.touch();

                info!(
                    thread_id = %updated.id,
                    visibility = ?updated.visibility,
                    "updating thread visibility"
                );

                thread_store.save(&updated).await
                    .context("failed to save thread with updated visibility")?;

                println!(
                    "Updated visibility of {} to {:?}.",
                    updated.id, updated.visibility
                );
            } else {
                anyhow::bail!("Either --visibility or --support must be specified");
            }

            Ok(())
        }
        None => {
            let thread = create_new_thread(&config, &args)?;
            start_repl_session(&config, &args, thread_store, thread).await
        }
    }
}

fn setup_ctrlc_handler(shutdown_flag: Arc<AtomicBool>) -> Result<()> {
    ctrlc::set_handler(move || {
        info!("received Ctrl+C, requesting shutdown");
        shutdown_flag.store(true, Ordering::Relaxed);
        // Print newline to clean up prompt
        eprintln!();
    })
    .context("failed to set Ctrl+C handler")?;

    Ok(())
}
