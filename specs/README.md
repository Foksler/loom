# Loom Specifications

This directory contains the architectural specifications and design documentation for **Loom**, an AI-powered coding agent written in Rust.

## Specification Index

| Document | Description |
|----------|-------------|
| [architecture.md](./architecture.md) | Overall system architecture, crate structure, and design patterns |
| [state-machine.md](./state-machine.md) | Agent state machine design with states, events, and transitions |
| [llm-client.md](./llm-client.md) | LLM client abstraction and provider implementations |
| [tool-system.md](./tool-system.md) | Tool registry, execution states, and built-in tools |
| [streaming.md](./streaming.md) | SSE streaming design for real-time LLM responses |
| [error-handling.md](./error-handling.md) | Error types, propagation, and recovery strategies |
| [retry-strategy.md](./retry-strategy.md) | HTTP retry/backoff with exponential backoff and jitter |
| [testing.md](./testing.md) | Testing strategy with property-based tests |
| [configuration.md](./configuration.md) | CLI arguments and basic config overview |
| [configuration-system.md](./configuration-system.md) | Full configuration registry, XDG paths, TOML format, layered sources |
| [distribution.md](./distribution.md) | Build system, multi-platform binaries, self-update, and CI/CD pipeline |
| [health-check.md](./health-check.md) | Health check endpoints for monitoring and load balancer integration |
| [thread-system.md](./thread-system.md) | Thread persistence, local storage, server sync, and CLI commands |
| [git-metadata.md](./git-metadata.md) | Git repository detection, loom-git crate, and thread git metadata |
| [search-system.md](./search-system.md) | Full-text search with FTS5, commit SHA lookup, and CLI search command |
| [secret-system.md](./secret-system.md) | Secret<T> type for safe handling of API keys and sensitive values |
| [acp-system.md](./acp-system.md) | Agent Client Protocol (ACP) integration for editor-driven usage |

## Quick Reference

### Crate Structure

```
loom/
├── crates/
│   ├── loom-secret/         # Secret<T> type for safe handling of sensitive values
│   ├── loom-config-common/  # Shared config primitives, re-exports loom-secret
│   ├── loom-config/         # Configuration management, XDG paths, layered sources
│   ├── loom-core/           # Core types, state machine, LLM traits
│   ├── loom-git/            # Git repository detection for threads
│   ├── loom-thread/         # Thread persistence and sync
│   ├── loom-server/         # HTTP server with SQLite
│   ├── loom-tools/          # Tool definitions (read_file, list_files, edit_file)
│   ├── loom-llm-anthropic/  # Anthropic Claude API client
│   ├── loom-llm-openai/     # OpenAI GPT API client
│   ├── loom-http-retry/     # HTTP retry/backoff utilities
│   ├── loom-acp/            # Agent Client Protocol (ACP) integration
│   └── loom-cli/            # CLI binary using clap
└── specs/                   # This directory
```

### Key Design Patterns

1. **State Machine** - Agent uses explicit state machine for conversation flow
2. **Strategy Pattern** - `LlmClient` trait allows swappable LLM providers
3. **Registry Pattern** - `ToolRegistry` for dynamic tool lookup
4. **Discriminated Unions** - Rust enums for type-safe state representation

### Extension Points

- **Add LLM Provider**: See [llm-client.md](./llm-client.md#adding-new-providers)
- **Add Tool**: See [tool-system.md](./tool-system.md#adding-new-tools)
- **Add Agent State**: See [state-machine.md](./state-machine.md#extension-guide)
- **Add Config Source**: See [configuration-system.md](./configuration-system.md#6-configuration-registry-architecture)

## Usage

These specifications are designed to:

1. **Document design decisions** for future reference
2. **Guide extensions** when adding new features
3. **Onboard contributors** to the codebase
4. **Maintain consistency** as the project evolves

When extending Loom, consult the relevant specification first to understand the existing patterns and conventions.
