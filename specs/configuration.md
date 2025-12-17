# Configuration Design

## Overview

Loom uses a layered configuration approach that balances flexibility with simplicity:

```
┌─────────────────────────────────────────────────────────────┐
│                     CLI Arguments                           │
│                  (highest precedence)                       │
├─────────────────────────────────────────────────────────────┤
│                 Environment Variables                       │
├─────────────────────────────────────────────────────────────┤
│                    Default Values                           │
│                  (lowest precedence)                        │
└─────────────────────────────────────────────────────────────┘
```

Configuration flows from three sources:
1. **CLI arguments** - Explicit user intent, highest precedence
2. **Environment variables** - Machine/session configuration
3. **Hardcoded defaults** - Sensible fallbacks

## CLI Arguments

Defined in [`crates/loom-cli/src/main.rs`](../crates/loom-cli/src/main.rs) using clap derive macros:

```rust
#[derive(Parser, Debug)]
#[command(name = "loom", version, about, long_about = None)]
struct Args {
    /// LLM provider to use
    #[arg(short, long, default_value = "anthropic")]
    provider: Provider,

    /// Model name (uses provider default if not specified)
    #[arg(short, long)]
    model: Option<String>,

    /// API key for the selected provider
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
```

### Argument Reference

| Argument | Short | Type | Default | Description |
|----------|-------|------|---------|-------------|
| `--provider` | `-p` | `anthropic \| openai` | `anthropic` | LLM provider selection |
| `--model` | `-m` | `String` | Provider default | Model name override |
| `--api-key` | - | `String` | From env | API key (env fallback) |
| `--workspace` | `-w` | `PathBuf` | `.` | Workspace directory |
| `--log-level` | `-l` | `trace \| debug \| info \| warn \| error` | `info` | Logging verbosity |
| `--json-logs` | - | `bool` | `false` | Structured JSON log output |

### Provider Enum

```rust
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum Provider {
    #[default]
    Anthropic,
    OpenAi,
}
```

### Log Level Enum

```rust
#[derive(Debug, Clone, Copy, Default, ValueEnum)]
enum LogLevel {
    Trace,
    Debug,
    #[default]
    Info,
    Warn,
    Error,
}
```

## Environment Variables

### API Keys

| Variable | Provider | Description |
|----------|----------|-------------|
| `ANTHROPIC_API_KEY` | Anthropic | Claude API authentication |
| `OPENAI_API_KEY` | OpenAI | GPT API authentication |

API key resolution logic in [`crates/loom-cli/src/main.rs`](../crates/loom-cli/src/main.rs):

```rust
fn resolve_api_key(&self) -> Result<String> {
    // CLI argument takes precedence
    if let Some(ref key) = self.api_key {
        return Ok(key.clone());
    }

    // Fall back to provider-specific env var
    let env_var = match self.provider {
        Provider::Anthropic => "ANTHROPIC_API_KEY",
        Provider::OpenAi => "OPENAI_API_KEY",
    };

    std::env::var(env_var)
        .with_context(|| format!("API key not provided. Set --api-key or {}", env_var))
}
```

### Logging

| Variable | Description |
|----------|-------------|
| `RUST_LOG` | tracing filter directive (overrides `--log-level`) |

The `RUST_LOG` environment variable uses tracing's `EnvFilter` syntax and takes precedence over `--log-level`:

```rust
let filter = EnvFilter::try_from_default_env()
    .unwrap_or_else(|_| EnvFilter::new(format!("loom={}", tracing::Level::from(log_level))));
```

Examples:
- `RUST_LOG=debug` - All debug logs
- `RUST_LOG=loom=trace,reqwest=warn` - Trace for loom, warn for reqwest
- `RUST_LOG=loom_llm_anthropic=debug` - Debug specific crate

## Configuration Structs

### AgentConfig

Core agent behavior configuration in [`crates/loom-core/src/config.rs`](../crates/loom-core/src/config.rs):

```rust
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct AgentConfig {
    pub model_name: String,
    pub max_retries: u32,
    #[serde(with = "humantime_serde")]
    pub tool_timeout: Duration,
    #[serde(with = "humantime_serde")]
    pub llm_timeout: Duration,
    pub max_tokens: u32,
    pub temperature: Option<f32>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            model_name: "claude-sonnet-4-20250514".to_string(),
            max_retries: 3,
            tool_timeout: Duration::from_secs(30),
            llm_timeout: Duration::from_secs(120),
            max_tokens: 4096,
            temperature: None,
        }
    }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `model_name` | `String` | `claude-sonnet-4-20250514` | Default model identifier |
| `max_retries` | `u32` | `3` | Maximum retry attempts |
| `tool_timeout` | `Duration` | `30s` | Tool execution timeout |
| `llm_timeout` | `Duration` | `120s` | LLM API request timeout |
| `max_tokens` | `u32` | `4096` | Maximum tokens in response |
| `temperature` | `Option<f32>` | `None` | Sampling temperature |

### AnthropicConfig

Anthropic client configuration in [`crates/loom-llm-anthropic/src/types.rs`](../crates/loom-llm-anthropic/src/types.rs):

```rust
#[derive(Debug, Clone)]
pub struct AnthropicConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | `String` | Required | Anthropic API key |
| `base_url` | `String` | `https://api.anthropic.com` | API endpoint |
| `model` | `String` | `claude-sonnet-4-20250514` | Model identifier |

### OpenAIConfig

OpenAI client configuration in [`crates/loom-llm-openai/src/types.rs`](../crates/loom-llm-openai/src/types.rs):

```rust
#[derive(Debug, Clone)]
pub struct OpenAIConfig {
    pub api_key: String,
    pub base_url: String,
    pub model: String,
    pub organization: Option<String>,
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `api_key` | `String` | Required | OpenAI API key |
| `base_url` | `String` | `https://api.openai.com/v1` | API endpoint |
| `model` | `String` | `gpt-4o` | Model identifier |
| `organization` | `Option<String>` | `None` | OpenAI organization ID |

### RetryConfig

HTTP retry behavior in [`crates/loom-http-retry/src/lib.rs`](../crates/loom-http-retry/src/lib.rs):

```rust
#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_attempts: u32,
    pub base_delay: Duration,
    pub max_delay: Duration,
    pub backoff_factor: f64,
    pub jitter: bool,
    pub retryable_statuses: Vec<StatusCode>,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            base_delay: Duration::from_millis(200),
            max_delay: Duration::from_secs(5),
            backoff_factor: 2.0,
            jitter: true,
            retryable_statuses: vec![
                StatusCode::TOO_MANY_REQUESTS,
                StatusCode::REQUEST_TIMEOUT,
                StatusCode::BAD_GATEWAY,
                StatusCode::SERVICE_UNAVAILABLE,
                StatusCode::GATEWAY_TIMEOUT,
            ],
        }
    }
}
```

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `max_attempts` | `u32` | `3` | Maximum retry attempts |
| `base_delay` | `Duration` | `200ms` | Initial delay before first retry |
| `max_delay` | `Duration` | `5s` | Maximum delay cap |
| `backoff_factor` | `f64` | `2.0` | Exponential backoff multiplier |
| `jitter` | `bool` | `true` | Add randomization to delays |
| `retryable_statuses` | `Vec<StatusCode>` | 429, 408, 502, 503, 504 | HTTP statuses that trigger retry |

## Builder Pattern

Provider configs use fluent builder pattern with `with_*` methods:

### AnthropicConfig Builder

```rust
impl AnthropicConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.anthropic.com".to_string(),
            model: "claude-sonnet-4-20250514".to_string(),
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }
}
```

### OpenAIConfig Builder

```rust
impl OpenAIConfig {
    pub fn new(api_key: impl Into<String>) -> Self {
        Self {
            api_key: api_key.into(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4o".to_string(),
            organization: None,
        }
    }

    pub fn with_base_url(mut self, base_url: impl Into<String>) -> Self {
        self.base_url = base_url.into();
        self
    }

    pub fn with_model(mut self, model: impl Into<String>) -> Self {
        self.model = model.into();
        self
    }

    pub fn with_organization(mut self, org: impl Into<String>) -> Self {
        self.organization = Some(org.into());
        self
    }
}
```

### Usage Example

```rust
// Basic usage
let config = AnthropicConfig::new(api_key);

// With customization
let config = AnthropicConfig::new(api_key)
    .with_model("claude-3-opus-20240229")
    .with_base_url("http://localhost:8080");

let client = AnthropicClient::new(config)?;
```

## Design Decisions

### Why clap derive over builder

We use clap's derive macros rather than the builder API because:

1. **Type safety**: Enum variants for providers and log levels are validated at compile time
2. **Documentation**: Doc comments become help text automatically
3. **Maintainability**: Adding new arguments requires minimal boilerplate
4. **Consistency**: Derive pattern matches our config struct style

### Environment Variable Precedence

The precedence order (CLI > env > default) was chosen because:

1. **Explicit intent**: CLI arguments represent immediate user intent
2. **Session config**: Env vars represent machine/session defaults
3. **Zero config**: Defaults allow running without any configuration
4. **Security**: API keys work seamlessly from env without appearing in command history

### Provider-Specific Env Vars

We check provider-specific env vars (`ANTHROPIC_API_KEY`, `OPENAI_API_KEY`) rather than a generic `API_KEY` because:

1. **Multi-provider**: Users may have both keys set for different use cases
2. **Convention**: Matches what LLM libraries and tools expect
3. **Clarity**: No ambiguity about which key is being used

### Sensible Defaults

Default values are chosen for common developer workflows:

| Setting | Default | Rationale |
|---------|---------|-----------|
| Provider | Anthropic | Claude excels at coding tasks |
| Log level | Info | Balance of visibility and noise |
| Workspace | `.` | Current directory is most common |
| Retry attempts | 3 | Handles transient failures without excessive delay |
| Jitter | enabled | Prevents thundering herd in concurrent usage |

## Future Extensions

### Config File Support

Add TOML/YAML configuration file support:

```toml
# ~/.config/loom/config.toml
[default]
provider = "anthropic"
model = "claude-sonnet-4-20250514"
log_level = "info"

[agent]
max_retries = 3
tool_timeout = "30s"
llm_timeout = "120s"
max_tokens = 4096

[anthropic]
base_url = "https://api.anthropic.com"

[openai]
base_url = "https://api.openai.com/v1"
organization = "org-xxx"
```

Loading precedence would become:
```
CLI > Environment > Config File > Defaults
```

### Per-Workspace Configuration

Support `.loom.toml` in workspace root for project-specific settings:

```toml
# /path/to/project/.loom.toml
model = "claude-3-opus-20240229"
max_tokens = 8192

[tools]
enabled = ["read_file", "edit_file", "list_files"]
```

### Profile Support

Named configuration profiles for different use cases:

```toml
[profiles.coding]
model = "claude-sonnet-4-20250514"
temperature = 0.0

[profiles.creative]
model = "claude-3-opus-20240229"
temperature = 0.8
```

Usage:
```bash
loom --profile coding
loom --profile creative
```

### Additional Future Options

| Feature | Description |
|---------|-------------|
| `--config` | Explicit config file path |
| `--dry-run` | Show effective configuration without running |
| Config validation | `loom config validate` subcommand |
| Config generation | `loom config init` to create template |
| Secret management | Integration with system keyring |
