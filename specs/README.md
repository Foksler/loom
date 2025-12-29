<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Loom Specifications

This directory contains the architectural specifications and design documentation for **Loom**, an
AI-powered coding agent written in Rust.

## Specification Index

| Document                                                       | Description                                                                         |
| -------------------------------------------------------------- | ----------------------------------------------------------------------------------- |
| [architecture.md](./architecture.md)                           | Overall system architecture, crate structure, and design patterns                   |
| [state-machine.md](./state-machine.md)                         | Agent state machine design with states, events, and transitions                     |
| [llm-client.md](./llm-client.md)                               | LLM client abstraction and provider implementations                                 |
| [tool-system.md](./tool-system.md)                             | Tool registry, execution states, and built-in tools                                 |
| [streaming.md](./streaming.md)                                 | SSE streaming design for real-time LLM responses                                    |
| [error-handling.md](./error-handling.md)                       | Error types, propagation, and recovery strategies                                   |
| [retry-strategy.md](./retry-strategy.md)                       | HTTP retry/backoff with exponential backoff and jitter                              |
| [testing.md](./testing.md)                                     | Testing strategy with property-based tests                                          |
| [configuration.md](./configuration.md)                         | CLI arguments and basic config overview                                             |
| [configuration-system.md](./configuration-system.md)           | Full configuration registry, XDG paths, TOML format, layered sources                |
| [distribution.md](./distribution.md)                           | Build system, multi-platform binaries, self-update, and CI/CD pipeline              |
| [sbom-system.md](./sbom-system.md)                             | Software Bill of Materials generation, formats (SPDX/CycloneDX), and CI integration |
| [container-system.md](./container-system.md)                   | Docker/OCI container builds via Nix/devenv, reproducible images, security hardening |
| [health-check.md](./health-check.md)                           | Health check endpoints for monitoring and load balancer integration                 |
| [thread-system.md](./thread-system.md)                         | Thread persistence, local storage, server sync, and CLI commands                    |
| [git-metadata.md](./git-metadata.md)                           | Git repository detection, loom-git crate, and thread git metadata                   |
| [search-system.md](./search-system.md)                         | Full-text search with FTS5, commit SHA lookup, and CLI search command               |
| [secret-system.md](./secret-system.md)                         | Secret<T> type for safe handling of API keys and sensitive values                   |
| [acp-system.md](./acp-system.md)                               | Agent Client Protocol (ACP) integration for editor-driven usage                     |
| [server-query-phase-2.md](./server-query-phase-2.md)           | Phase 2: Server-to-Client Query Bridge LLM Integration                              |
| [phase3_websocket_planning.md](./phase3_websocket_planning.md) | Phase 3: WebSocket upgrade for persistent connections                               |
| [api-documentation.md](./api-documentation.md)                 | OpenAPI documentation system using utoipa and Swagger UI                            |
| [vscode-extension.md](./vscode-extension.md)                   | VS Code extension (loom-vscode) using ACP for editor integration                    |
| [weaver-provisioner.md](./weaver-provisioner.md)               | K8s weaver provisioner for ephemeral execution environments                         |

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
│   ├── loom-http/           # Shared HTTP client and retry utilities
│   ├── loom-acp/            # Agent Client Protocol (ACP) integration
│   ├── loom-k8s/            # Kubernetes client abstraction
│   ├── loom-weaver/         # K8s weaver provisioning and lifecycle
│   └── loom-cli/            # CLI binary using clap
├── ide/
│   └── vscode/              # VS Code extension (ACP client)
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
- **Add Config Source**: See
  [configuration-system.md](./configuration-system.md#6-configuration-registry-architecture)

## Phase 2: Query Bridge Enhancements

### New Documentation (Query Bridge Focus)

The Query Bridge feature includes comprehensive production guides:

- **[INTEGRATION_GUIDE.md](../INTEGRATION_GUIDE.md)** - How to integrate Query Bridge into existing
  LLM loop
- **[PERFORMANCE_TUNING.md](../PERFORMANCE_TUNING.md)** - Latency optimization, timeout settings,
  detection tuning
- **[SECURITY_HARDENING.md](../SECURITY_HARDENING.md)** - Security configuration, audit logging,
  rate limiting

See [INDEX_QUERY_BRIDGE.md](../INDEX_QUERY_BRIDGE.md) for complete documentation index.

## Usage

These specifications are designed to:

1. **Document design decisions** for future reference
2. **Guide extensions** when adding new features
3. **Onboard contributors** to the codebase
4. **Maintain consistency** as the project evolves

When extending Loom, consult the relevant specification first to understand the existing patterns
and conventions.

## Phase Roadmap

| Phase        | Status      | Focus                                            | Docs                                                                    |
| ------------ | ----------- | ------------------------------------------------ | ----------------------------------------------------------------------- |
| **Phase 1**  | ✅ Complete | Core framework, types, manager, handler          | [Implementation Guide](../IMPLEMENTATION_SERVER_CLIENT_QUERY_BRIDGE.md) |
| **Phase 2**  | ✅ Complete | LLM integration, extraction, context restoration | [Phase 2 Guide](../PHASE_2_IMPLEMENTATION_GUIDE.md)                     |
| **Phase 3**  | 📋 Planned  | WebSocket upgrade, persistent connections        | [WebSocket Planning](./phase3_websocket_planning.md)                    |
| **Phase 4+** | 📋 Future   | Editor integration, advanced query types         | TBD                                                                     |
| **VS Code**  | 🚧 WIP      | VS Code extension using ACP                      | [VS Code Extension](./vscode-extension.md)                              |
