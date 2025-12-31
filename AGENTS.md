# Loom Agent Guidelines

## Commands
- **Build:** `cargo build --workspace`
- **Test all:** `cargo test --workspace`
- **Test single:** `cargo test -p loom-<crate> <test_name>` (e.g., `cargo test -p loom-core test_agent`)
- **Lint:** `cargo clippy --workspace -- -D warnings`
- **Format:** `cargo fmt --all`
- **Check all:** `make check` (format + lint + build + test)
- **Web dev:** `cd web/loom-web && pnpm dev` | **Web test:** `pnpm test`

## Deployment
Deployments happen automatically via `git push` to the `trunk` branch. The production server runs NixOS with auto-update enabled.

- **Deploy:** `git push origin trunk`
- **Check status:** `systemctl status nixos-auto-update.service`
- **View logs:** `journalctl -u nixos-auto-update.service -f` (follow) or `-n 100` (last 100 lines)
- **Service state:** `activating` = deploying, `active (exited)` = completed successfully

## Local Testing
Before deploying, test changes locally to verify behavior:

- **Run server on alternate port:** `LOOM_SERVER_PORT=9090 LOOM_SERVER_DB_PATH=/tmp/loom-test.db ./target/release/loom-server`
- **Dev mode (auto-auth):** Add `LOOM_SERVER_AUTH_DEV_MODE=1` for testing without real auth
- **Test against local:** `curl http://localhost:9090/health`
- **Run integration tests:** `cargo test -p loom-server <test_name>`

## Architecture
Rust workspace with 30+ crates under `crates/`. Key crates: `loom-core` (agent logic), `loom-server` (HTTP API), `loom-thread` (conversation state), `loom-llm-*` (LLM providers), `loom-tools` (agent tools), `loom-auth*` (authentication). Web frontend in `web/loom-web` (SvelteKit + Tailwind). SQLite database (`sqlx`). Dev environment via `devenv.nix`. Infra in `infra/` (Nix/K8s).

## Svelte 5 (NOT Svelte 4)
**Always use Svelte 5 runes syntax. Never use Svelte 4 patterns.**

| Category | ✅ Svelte 5 | ❌ Svelte 4 (DO NOT USE) |
|----------|-------------|--------------------------|
| **State** | `let count = $state(0);` | `let count = 0;` |
| **Derived** | `const doubled = $derived(count * 2);` | `$: doubled = count * 2;` |
| **Effects** | `$effect(() => { ... });` | `$: { ... }` |
| **Props** | `let { foo, bar } = $props();` | `export let foo;` |
| **Events** | `onclick={handler}` | `on:click={handler}` |
| **Custom events** | Pass callback props: `onsave={fn}` | `createEventDispatcher` |
| **Slots** | `{@render children()}` | `<slot />` |

Stores (`writable`, `$store`) are supported but prefer runes for component state.

## Code Style
- **Formatting:** Hard tabs, 2-space width, 100 char line width (rustfmt.toml)
- **Errors:** Use `thiserror` for error enums, `anyhow` for propagation. Define `Result<T>` type aliases.
- **Async:** Tokio runtime. Use `async-trait` for async trait methods.
- **Imports:** Group std, external crates, then internal `loom-*` crates.
- **Naming:** snake_case for functions/variables, PascalCase for types, SCREAMING_CASE for constants.
- **No comments** unless code is complex and requires context for future developers. Copyright header required.
- **HTTP clients:** Never build `reqwest::Client` directly. Use `loom-http::{new_client, builder}` for consistent User-Agent and retry logic.
- **Testing:** Prefer property-based tests (`proptest`) over unit tests when appropriate; use unit tests for simple cases.
- **Logging:** Use structured logging (`tracing`). Never log secrets directly.
- **Instrumentation:** Use `#[instrument(skip(self, secrets, large_args), fields(id = %id))]`. Always skip secrets.
- **Secrets:** Use `loom-secret::{Secret, SecretString}` for API keys, tokens, passwords. Access via `.expose()`. Auto-redacts in Debug/Display/Serialize/tracing.

## Internationalization (i18n)

### Crate
Use `loom-i18n` for all translatable strings. Uses GNU gettext with `.po` files compiled to `.mo` at build time.

### String Naming Convention
All translatable strings use hierarchical dot-notation: `{prefix}.{domain}.{component}.{element}`

| Prefix | Usage | Example |
|--------|-------|---------|
| `server.` | Backend strings (emails, API responses) | `server.email.magic_link.subject` |
| `client.` | CLI strings (loom-cli output) | `client.error.connection_failed` |

### Domains
- `email` - Email subjects and bodies
- `api` - API response messages
- `auth` - Authentication messages
- `org` - Organization-related messages

### Usage
```rust
use loom_i18n::{t, t_fmt, is_rtl, resolve_locale};

// Simple translation
let subject = t("es", "server.email.magic_link.subject");

// Translation with variables (use {name} syntax)
let body = t_fmt("es", "server.email.invitation.subject", &[
    ("org_name", "Acme Corp"),
]);

// Resolve locale: user preference → server default → "en"
let locale = resolve_locale(user.locale.as_deref(), &config.default_locale);

// RTL support for HTML emails
if is_rtl(locale) {
    // Use dir="rtl" in HTML
}
```

### Adding Translations
1. Add msgid/msgstr to `crates/loom-i18n/locales/{locale}/messages.po`
2. Run `cargo build -p loom-i18n` to compile `.po` → `.mo`
3. Supported locales: `en` (English), `es` (Spanish), `ar` (Arabic/RTL)

### RTL Languages
Arabic (`ar`) and other RTL locales require `dir="rtl"` on HTML elements. Use `loom_i18n::is_rtl()` to check.
