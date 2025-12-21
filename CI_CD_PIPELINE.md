# CI/CD Pipeline Documentation

Complete guide to the Loom GitHub Actions CI/CD pipeline and development environment.

## Table of Contents

- [Pipeline Overview](#pipeline-overview)
- [Jobs & Stages](#jobs--stages)
- [Local Development](#local-development)
- [Deployment](#deployment)
- [Monitoring & Debugging](#monitoring--debugging)
- [Configuration](#configuration)

## Pipeline Overview

### Trigger Conditions

The CI/CD pipeline automatically runs when:

- ✅ Code is pushed to `main` or `master` branch
- ✅ A pull request is opened against `main` or `master`
- ❌ NOT triggered on other branches or draft PRs

### Concurrency

- Only one workflow per pull request runs at a time
- Previous runs for the same PR are cancelled
- Prevents resource waste and ensures clean results

### Caching

- Rust build cache is maintained between runs
- Cargo cache speeds up subsequent builds by 60-80%
- Cache is invalidated on dependency changes

## Jobs & Stages

### 1. Formatting Check (`fmt`)

**Name**: Rustfmt

**Purpose**: Ensure consistent code style

```bash
cargo fmt --all -- --check
```

**Time**: ~10 seconds
**Failure**: Blocks merge
**Fix**: Run `cargo fmt --all` locally

---

### 2. Linting (`clippy`)

**Name**: Clippy

**Purpose**: Detect potential bugs and style issues

```bash
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

**Time**: ~2-3 minutes
**Failure**: Blocks merge
**Fix**: Run `cargo clippy --fix` or address warnings manually

**Common Issues**:
```
warning: this clone is inefficient
warning: use of `clone` on a `Vec` when you could use `to_vec()`
warning: this is a simple `try!` block
```

---

### 3. Cargo Check

**Name**: Check

**Purpose**: Verify code compiles with stable & nightly

**Matrix**:
- Stable Rust
- Nightly Rust

```bash
cargo check --workspace --all-targets
```

**Time**: ~1-2 minutes each
**Failure**: Blocks merge
**Note**: Nightly failures are informational but don't block merge

---

### 4. Unit Tests (`test`)

**Name**: Test Suite

**Purpose**: Run unit and documentation tests

```bash
cargo test --workspace --lib --bins
cargo test --doc --workspace
```

**Time**: ~2-3 minutes
**Failure**: Blocks merge

**Coverage**: Check with `cargo tarpaulin`

---

### 5. Integration Tests (`integration-tests`)

**Name**: Integration Tests

**Purpose**: Test crates working together

```bash
cargo test --test '*'
```

**Time**: ~1-2 minutes
**Failure**: Blocks merge
**Timeout**: 10 minutes (kills hung tests)

---

### 6. Security Audit (`security-audit`)

**Name**: Security Audit

**Purpose**: Check for known vulnerabilities in dependencies

Uses: `rustsec/audit-check-action`

**Time**: ~30 seconds
**Failure**: Blocks merge
**Fix**: Update vulnerable dependencies or file security exception

---

### 7. Build Release (`build`)

**Name**: Build

**Purpose**: Full release build of all binaries

```bash
cargo build --workspace --release
```

**Time**: ~3-5 minutes
**Failure**: Blocks merge
**Output**: Release binaries in `target/release/`

---

### 8. Build Web UI (`build-web`)

**Name**: Build Web UI

**Purpose**: Build Leptos web application

**Requirements**:
- Node.js 18+
- npm

**Steps**:
```bash
cd crates/loom-web
npm ci                # Clean install
npm run build         # Production build
```

**Time**: ~2-3 minutes
**Failure**: Blocks merge
**Output**: Optimized web bundle in `crates/loom-web/target/`

---

### 9. Dependency Check (`dependencies`)

**Name**: Dependency Check

**Purpose**: Audit dependencies for security and compatibility

Uses: `cargo-deny`

**Checks**:
- License compliance
- Security vulnerabilities
- Unmaintained crates
- Duplicate dependencies

**Time**: ~30 seconds
**Failure**: Blocks merge
**Config**: `.cargo-deny.toml` or `deny.toml`

---

### 10. Unused Dependencies (`unused-deps`)

**Name**: Unused Dependencies

**Purpose**: Detect dependencies not actually used

Uses: `cargo +nightly udeps`

**Time**: ~2 minutes
**Failure**: Informational only (doesn't block)
**Note**: Requires nightly Rust

---

### 11. Documentation (`docs`)

**Name**: Documentation

**Purpose**: Verify documentation compiles and is valid

```bash
cargo doc --no-deps --document-private-items
```

**Flags**:
- `-D warnings`: Treat warnings as errors
- `--no-deps`: Don't build dependency docs
- `--document-private-items`: Include private items

**Time**: ~1-2 minutes
**Failure**: Blocks merge

**Requirements**:
- Doc comments on public items
- Valid examples in doc comments
- Correct cross-references

---

### 12. CI Complete (`ci-complete`)

**Name**: CI Complete

**Purpose**: Final gating - ensures all required checks passed

**Dependencies**: All above jobs

**Time**: ~10 seconds
**Failure**: Blocks merge if any job failed

**This is the final status check GitHub uses for branch protection rules.**

---

## Local Development

### Running Locally Before Push

```bash
# Full CI suite (recommended before pushing)
make check

# Individual checks
make build      # cargo build
make test       # cargo test
make lint       # cargo clippy
make format     # cargo fmt
make check-format  # fmt check only
```

### Matching CI Environment

```bash
# Install latest Rust stable
rustup update stable

# Install Nightly (for some checks)
rustup toolchain install nightly

# Install clippy component
rustup component add clippy

# Verify Rust versions
rustc --version     # Should be >= 1.70
cargo +nightly --version

# Install tools if needed
cargo install cargo-clippy
cargo install cargo-deny
```

### Speeding Up Local Builds

```bash
# Use all available cores
export CARGO_BUILD_JOBS=$(nproc)

# Enable incremental compilation
export CARGO_INCREMENTAL=1

# Build in parallel
cargo build -j $(nproc)

# For faster clippy checks
cargo clippy --lib  # Skip examples/tests
```

### Pre-commit Hook

Auto-run checks before committing:

```bash
# Create .git/hooks/pre-commit
#!/bin/bash
set -e

# Run format check
cargo fmt --all -- --check || {
    echo "Format check failed. Run: cargo fmt --all"
    exit 1
}

# Run clippy
cargo clippy --workspace -- -D warnings || {
    echo "Clippy check failed. Address warnings above."
    exit 1
}

# Run quick tests
cargo test --lib || {
    echo "Tests failed."
    exit 1
}

echo "✓ Pre-commit checks passed"
```

Make it executable:
```bash
chmod +x .git/hooks/pre-commit
```

## Deployment

### Release Process

CI pipeline supports:

1. **Automatic releases**: Tagged commits
2. **Manual releases**: Triggered via workflow dispatch
3. **Nightly builds**: Continuous prerelease builds

### Artifacts

Generated by CI:

- **Release binaries**: `target/release/loom-*`
- **Web bundle**: `crates/loom-web/dist/`
- **Documentation**: Generated docs
- **SBOM**: Software Bill of Materials

### Docker Images

Built via Nix/Makefile:

```bash
# Local
make docker-build   # Build image
make docker-run     # Run locally

# In CI (if configured)
# Images pushed to Docker registry
```

## Monitoring & Debugging

### Viewing CI Results

1. **GitHub UI**: Actions tab → Workflow runs
2. **PR checks**: "Checks" tab on PR
3. **Failing jobs**: Click job name for logs

### Debugging Failed Jobs

#### Format Issues
```bash
# See what cargo fmt would change
cargo fmt --all -- --check --diff
```

#### Clippy Warnings
```bash
# Run locally with same settings
cargo clippy --workspace --all-targets -- -D warnings
```

#### Test Failures
```bash
# Run same test locally with output
RUST_LOG=debug cargo test test_name -- --nocapture
```

#### Build Failures
```bash
# Clean build to rule out cache issues
cargo clean
cargo build --all
```

#### Security Audit Failures
```bash
# Check vulnerable dependencies
cargo audit

# See all dependencies
cargo tree

# Update to fix versions
cargo update
```

### Common CI Failures

| Problem | Solution |
|---------|----------|
| Format errors | Run `cargo fmt --all` |
| Clippy warnings | Run `cargo clippy --fix` or fix manually |
| Test failures | Run `cargo test` locally |
| Dependency conflicts | Run `cargo update` |
| Out of disk | Check `/tmp` and `target/` size |
| Timeout | Break test into smaller pieces |
| Nightly regression | File issue with nightly team |

### Accessing CI Logs

```bash
# Via GitHub CLI
gh run view <run-id>

# Via GitHub web UI
1. Go to Actions tab
2. Click workflow name
3. Click failed job
4. See logs in details
```

### Debugging Locally with Same Setup

```bash
# Match CI Rust version
rustup default stable
rustup update

# Match CI Node version (if testing web)
nvm use 18

# Run same cargo command as CI
cargo clippy --workspace --all-targets --all-features -- -D warnings
```

## Configuration

### GitHub Actions Workflow

**File**: `.github/workflows/ci.yml`

**Key Variables**:
```yaml
env:
  CARGO_TERM_COLOR: always
  RUST_BACKTRACE: 1
  RUST_LOG: debug
```

### Concurrency Settings

```yaml
concurrency:
  group: ${{ github.workflow }}-${{ github.event.pull_request.number || github.ref }}
  cancel-in-progress: true
```

- Cancels previous runs for same PR
- Allows fresh runs if you push updates

### Matrix Strategy

```yaml
strategy:
  matrix:
    rust: [stable, nightly]
```

Runs check on both Rust versions (nightly failures don't block)

### Cache Configuration

```yaml
- uses: Swatinem/rust-cache@v2
  with:
    cache-on-failure: "true"
```

- Saves between runs
- Restored even if build fails
- Saves ~2 minutes per job

### Conditional Skipping

Skip specific jobs (not recommended, but possible):

```yaml
if: "!contains(github.event.head_commit.message, '[skip ci]')"
```

Use `[skip ci]` in commit message to skip entire workflow

### Branch Protection Rules

Set in Settings → Branches → Branch protection rules:

```
Require status checks to pass before merging:
✓ Rustfmt
✓ Clippy
✓ Check
✓ Test Suite
✓ Integration Tests
✓ Security Audit
✓ Build
✓ Build Web UI
✓ Dependency Check
✓ Documentation
✓ CI Complete
```

## Best Practices

1. **Run locally first**: Don't rely on CI to catch everything
2. **Keep commits clean**: Fix issues before pushing
3. **Monitor logs**: Learn from CI failures
4. **Update dependencies**: Keep security audit passing
5. **Test changes**: Don't skip tests
6. **Document changes**: Keep docs up to date
7. **Review PRs carefully**: CI catches some issues, reviewers catch others

## Advanced Topics

### Custom Workflows

Add workflows for:
- Scheduled security audits
- Performance benchmarks
- Deploy to staging/production
- Nightly release builds

Example addition to `.github/workflows/`:
```yaml
name: Nightly Builds

on:
  schedule:
    - cron: '0 2 * * *'

jobs:
  build:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@nightly
      - run: cargo build --release
```

### Secrets Management

For sensitive data:
1. Go to Settings → Secrets and variables → Actions
2. Create secret: `GITHUB_TOKEN` (auto-created)
3. Use in workflow: `${{ secrets.GITHUB_TOKEN }}`

### Performance Optimization

- Use `--lib` to skip examples in quick checks
- Use `--no-default-features` for quick builds
- Cache more aggressively
- Run slower jobs in parallel

## Troubleshooting

### "Workflow Syntax" Error

Check YAML indentation - GitHub Actions is strict

### Timeout

- Increase timeout in workflow
- Split large tests into smaller jobs
- Use `--lib` to skip integration tests

### Cache Not Working

```bash
# Clear cache
gh actions-cache delete <cache-key> -R <repo>

# Or wait for automatic expiration (7 days)
```

### Secrets Not Available

- Check secret is created in Settings
- Use `secrets.SECRET_NAME` (not environment variable)
- Secrets don't print in logs

---

## Quick Reference

```bash
# Before pushing
make check              # Full CI suite

# Individual checks
make build
make test
make lint
make format
make check-format

# Fix issues
cargo fmt --all        # Fix format
cargo clippy --fix     # Fix clippy issues
cargo test             # Verify tests pass
```

---

For more information, see:
- GitHub Actions Docs: https://docs.github.com/actions
- Rust CI Best Practices: https://docs.rs/book/
- Local Development: [DEV_ENVIRONMENT_SETUP.md](DEV_ENVIRONMENT_SETUP.md)
