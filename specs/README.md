<!--
 Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
 SPDX-License-Identifier: Proprietary
-->

# Loom Specifications

Design documentation for Loom, an AI-powered coding agent in Rust.

## Core Architecture

| Spec | Code | Purpose |
|------|------|---------|
| [architecture.md](./architecture.md) | [crates/](../crates/) | Crate structure, server-side LLM proxy design |
| [state-machine.md](./state-machine.md) | [loom-core](../crates/loom-core/) | Agent state machine for conversation flow |
| [tool-system.md](./tool-system.md) | [loom-tools](../crates/loom-tools/) | Tool registry and execution framework |
| [thread-system.md](./thread-system.md) | [loom-thread](../crates/loom-thread/) | Thread persistence and sync |
| [streaming.md](./streaming.md) | [loom-llm-service](../crates/loom-llm-service/) | SSE streaming for real-time LLM responses |
| [error-handling.md](./error-handling.md) | [loom-core](../crates/loom-core/) | Error types using `thiserror` |

## LLM Integration

| Spec | Code | Purpose |
|------|------|---------|
| [llm-client.md](./llm-client.md) | [loom-llm-anthropic](../crates/loom-llm-anthropic/), [loom-llm-openai](../crates/loom-llm-openai/) | `LlmClient` trait for providers |
| [anthropic-oauth-pool.md](./anthropic-oauth-pool.md) | [loom-llm-anthropic](../crates/loom-llm-anthropic/) | Claude subscription pooling with failover |
| [anthropic-max-pool-management.md](./anthropic-max-pool-management.md) | [loom-server](../crates/loom-server/) | Admin UI for OAuth pool management |
| [claude-subscription-auth.md](./claude-subscription-auth.md) | [loom-llm-anthropic](../crates/loom-llm-anthropic/) | OAuth 2.0 PKCE for Claude Pro/Max |
| [server-query-phase-2.md](./server-query-phase-2.md) | [loom-llm-proxy](../crates/loom-llm-proxy/) | LLM query detection and context injection |
| [phase3_websocket_planning.md](./phase3_websocket_planning.md) | [loom-server](../crates/loom-server/) | WebSocket upgrade planning |

## Configuration & Security

| Spec | Code | Purpose |
|------|------|---------|
| [configuration-system.md](./configuration-system.md) | [loom-config](../crates/loom-config/) | Layered config with XDG paths |
| [configuration.md](./configuration.md) | [loom-config-common](../crates/loom-config-common/) | CLI args and env vars |
| [secret-system.md](./secret-system.md) | [loom-secret](../crates/loom-secret/) | `Secret<T>` wrapper for sensitive values |
| [redact-system.md](./redact-system.md) | [loom-redact](../crates/loom-redact/) | Secret detection using gitleaks patterns |
| [auth-abac-system.md](./auth-abac-system.md) | [loom-auth](../crates/loom-auth/), [loom-auth-*](../crates/) | OAuth, magic links, ABAC |

## Server & API

| Spec | Code | Purpose |
|------|------|---------|
| [api-documentation.md](./api-documentation.md) | [loom-server](../crates/loom-server/) | OpenAPI docs with Swagger UI |
| [health-check.md](./health-check.md) | [loom-server](../crates/loom-server/) | `/health` endpoint |
| [retry-strategy.md](./retry-strategy.md) | [loom-http](../crates/loom-http/) | Exponential backoff |
| [job-scheduler-system.md](./job-scheduler-system.md) | [loom-jobs](../crates/loom-jobs/) | Background job system |

## Editor Integration

| Spec | Code | Purpose |
|------|------|---------|
| [acp-system.md](./acp-system.md) | [loom-acp](../crates/loom-acp/) | Agent Client Protocol for editors |
| [vscode-extension.md](./vscode-extension.md) | [ide/vscode](../ide/vscode/) | VS Code extension via ACP |

## SCM (Git Hosting)

| Spec | Code | Purpose |
|------|------|---------|
| [scm-system.md](./scm-system.md) | loom-scm (planned) | Git hosting via HTTPS, gitoxide, mirroring, webhooks |

## Git & Search

| Spec | Code | Purpose |
|------|------|---------|
| [git-metadata.md](./git-metadata.md) | [loom-git](../crates/loom-git/) | Git context tracking in threads |
| [auto-commit-system.md](./auto-commit-system.md) | [loom-auto-commit](../crates/loom-auto-commit/) | Auto-commit after tool execution |
| [search-system.md](./search-system.md) | [loom-thread](../crates/loom-thread/) | FTS5 search for threads |
| [web-search-system.md](./web-search-system.md) | [loom-google-cse](../crates/loom-google-cse/) | Google CSE integration |
| [github-app-system.md](./github-app-system.md) | [loom-github-app](../crates/loom-github-app/) | GitHub App for API access |

## Weaver (Remote Execution)

| Spec | Code | Purpose |
|------|------|---------|
| [weaver-provisioner.md](./weaver-provisioner.md) | [loom-weaver](../crates/loom-weaver/), [loom-k8s](../crates/loom-k8s/) | K8s pod provisioning |
| [weaver-cli.md](./weaver-cli.md) | [loom-cli](../crates/loom-cli/) | CLI for weaver management |

## Web, Distribution & Other

| Spec | Code | Purpose |
|------|------|---------|
| [loom-web.md](./loom-web.md) | [web/loom-web](../web/loom-web/) | Svelte 5 web frontend |
| [distribution.md](./distribution.md) | [loom-version](../crates/loom-version/) | Binary builds and self-update |
| [container-system.md](./container-system.md) | [docker/](../docker/), [flake.nix](../flake.nix) | Docker/OCI via Nix |
| [sbom-system.md](./sbom-system.md) | [.github/](../.github/) | SBOM generation (SPDX/CycloneDX) |
| [i18n-system.md](./i18n-system.md) | [loom-i18n](../crates/loom-i18n/) | Internationalization with gettext |
| [testing.md](./testing.md) | [crates/](../crates/) | Property-based testing with proptest |
