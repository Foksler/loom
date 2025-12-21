# Development Environment Setup Guide

Complete guide for setting up the Loom development environment.

## Table of Contents

- [System Requirements](#system-requirements)
- [Installation Methods](#installation-methods)
- [Workspace Setup](#workspace-setup)
- [IDE Configuration](#ide-configuration)
- [Verification](#verification)
- [Common Issues & Solutions](#common-issues--solutions)
- [Development Commands Reference](#development-commands-reference)

## System Requirements

### Core Requirements

- **Rust**: 1.70+ (stable)
- **Node.js**: 18+ (for web development)
- **Git**: 2.40+
- **Make**: GNU Make 4.0+
- **Cargo**: Latest stable

### Operating Systems

- **Linux**: Ubuntu 20.04+, Debian 11+, Fedora 36+, Arch
- **macOS**: 11+
- **Windows**: WSL2 or native with Visual Studio Build Tools

### Optional (Recommended)

- **direnv**: Automatic environment loading
- **Nix**: Reproducible development environment (via flakes)
- **Docker**: For containerized development
- **VS Code**: Development editor

## Installation Methods

### Method 1: Quick Start (Linux/macOS)

```bash
# Clone repository
git clone https://github.com/ghuntley/loom.git
cd loom

# Install Rust (if not installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source "$HOME/.cargo/env"

# Install Node.js (if not installed)
curl -o- https://raw.githubusercontent.com/nvm-sh/nvm/v0.39.0/install.sh | bash
nvm install 18
nvm use 18

# Verify installation
rustc --version
cargo --version
node --version
npm --version

# Build workspace
make build

# Run tests
make test
```

### Method 2: With Nix (Recommended)

**Prerequisites**: Nix 2.10+

```bash
# Clone repository
git clone https://github.com/ghuntley/loom.git
cd loom

# Enable flakes (one-time setup)
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Option A: Using direnv (automatic shell loading)
direnv allow
# Shell will auto-load with all dependencies

# Option B: Using nix shell
nix flake update
nix develop

# Now in nix shell - build the project
make build
make test
```

### Method 3: Manual Installation (Windows)

```powershell
# Install Rust
Invoke-WebRequest https://win.rustup.rs -OutFile rustup-init.exe
.\rustup-init.exe

# Install Node.js from https://nodejs.org/

# Close and reopen PowerShell

# Clone repository
git clone https://github.com/ghuntley/loom.git
cd loom

# Verify installation
rustc --version
cargo --version
node --version
npm --version

# Build workspace
cargo build --all
cargo test --all
```

## Workspace Setup

### Post-Installation

```bash
# Navigate to workspace
cd loom

# Install workspace dependencies
cargo fetch

# For web development, install npm dependencies
cd crates/loom-web
npm install
cd ../..

# Run initial verification
make check
```

### Workspace Structure

```
loom/
├── crates/                          # All Rust crates
│   ├── loom-core/                   # Core functionality
│   ├── loom-server/                 # Server implementation
│   ├── loom-web/                    # Web UI (Leptos + Rust)
│   ├── loom-cli/                    # CLI tool
│   ├── loom-llm-*                   # LLM provider crates
│   └── ...                          # Other specialized crates
├── examples/                        # Example code
├── tests/                           # Integration tests
├── nix/                             # Nix flake configurations
├── scripts/                         # Build and utility scripts
├── .github/workflows/               # GitHub Actions CI/CD
├── Makefile                         # Development tasks
├── Cargo.toml                       # Workspace manifest
├── devenv.nix                       # Development environment (if using devenv)
└── flake.nix                        # Nix flake configuration
```

## IDE Configuration

### VS Code Setup

**Recommended Extensions**:

1. **Rust-analyzer**
   - Extension: `rust-lang.rust-analyzer`
   - Provides IDE features for Rust

2. **CodeLLDB**
   - Extension: `vadimcn.vscode-lldb`
   - Debug support for Rust

3. **Leptos**
   - Extension: `leptos-community.leptos-lsp`
   - Support for Leptos framework

4. **Typical**
   - Extension: `tamasfe.even-better-toml`
   - TOML syntax highlighting

5. **Prettier**
   - Extension: `esbenp.prettier-vscode`
   - Code formatting

**Configuration** (`.vscode/settings.json`):

```json
{
  "[rust]": {
    "editor.formatOnSave": true,
    "editor.defaultFormatter": "rust-lang.rust-analyzer"
  },
  "rust-analyzer.checkOnSave.command": "clippy",
  "rust-analyzer.checkOnSave.extraArgs": ["--all", "--", "-D", "warnings"],
  "files.exclude": {
    "**/target": true
  }
}
```

**Launch Configuration** (`.vscode/launch.json`):

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "lldb",
      "request": "launch",
      "name": "Debug loom-cli",
      "cargo": {
        "args": [
          "build",
          "--package=loom-cli",
          "--bin=loom-cli",
          "--debug"
        ]
      },
      "args": [],
      "cwd": "${workspaceFolder}"
    }
  ]
}
```

### Vim/Neovim Setup

1. **Install rust-analyzer**:
   ```bash
   rustup component add rust-analyzer
   ```

2. **LSP Configuration** (using nvim-lspconfig):
   ```vim
   require('lspconfig').rust_analyzer.setup({
     settings = {
       ['rust-analyzer'] = {
         diagnostics = {
           enable = true,
         },
       },
     },
   })
   ```

3. **Format on save**:
   ```vim
   autocmd BufWritePre *.rs lua vim.lsp.buf.format()
   ```

### JetBrains IDEs (IntelliJ IDEA, CLion, RustRover)

1. Install **Rust** plugin from JetBrains marketplace
2. Enable macro expansion: Settings → Languages & Frameworks → Rust → Macro expansion
3. Format on save: Settings → Tools → Rust → Enable Rustfmt on Save

## Verification

### Quick Verification

```bash
# All components present
rustc --version
cargo --version
node --version
npm --version
make --version

# Build all crates
make build

# Run all tests
make test

# Check formatting
make check-format

# Run linter
make lint
```

### Running Specific Components

```bash
# Test specific crate
cargo test -p loom-core

# Build release binary
cargo build --release -p loom-cli

# Run CLI tool
./target/release/loom-cli --version

# Build web UI
cd crates/loom-web
npm run build
cd ../..

# Run E2E tests
make test-e2e
```

## Common Issues & Solutions

### Issue: Cargo build fails with SSL certificate error

**Solution**:
```bash
# Update certificates
rustup update

# Or use HTTP (not recommended for security)
cargo config set net.git-fetch-with-cli = true
```

### Issue: Node.js version mismatch

**Solution**:
```bash
# List available Node versions
nvm list-remote

# Install specific version
nvm install 18.19.0
nvm use 18.19.0

# Set default
nvm alias default 18
```

### Issue: `cargo build` fails with "linker not found"

**Solution (Linux)**:
```bash
# Ubuntu/Debian
sudo apt-get install build-essential

# Fedora
sudo dnf install gcc g++ make

# Arch
sudo pacman -S base-devel
```

**Solution (macOS)**:
```bash
xcode-select --install
```

**Solution (Windows)**:
Download and install Visual Studio Build Tools from: https://visualstudio.microsoft.com/visual-cpp-build-tools/

### Issue: Disk space after `cargo build`

The target directory can grow large. Clean periodically:

```bash
# Remove all build artifacts
cargo clean

# Or use the Makefile
make clean
```

### Issue: Permission denied when running scripts

**Solution**:
```bash
# Make scripts executable
chmod +x scripts/*.sh
```

### Issue: Tests fail with "too many open files"

**Solution**:
```bash
# Increase file descriptor limit
ulimit -n 4096

# Or permanently (add to ~/.bashrc or ~/.zshrc)
echo "ulimit -n 4096" >> ~/.bashrc
source ~/.bashrc
```

### Issue: Nix flakes not working

**Solution**:
```bash
# Update Nix to latest version
nix-channel --update

# Enable flakes in nix.conf
mkdir -p ~/.config/nix
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Or use direnv
direnv allow
```

## Development Commands Reference

### Using Makefile (Recommended)

```bash
# Build entire workspace
make build

# Run all tests
make test

# Run linter (clippy)
make lint

# Auto-fix issues and format
make fix

# Format code only
make format

# Check format (without modifying)
make check-format

# Full CI checks (format + lint + build + test)
make check

# Clean build artifacts
make clean

# Release build with SBOM
make release

# Docker image management
make docker-build     # Build image
make docker-run       # Build and run container

# E2E testing
make test-e2e         # Run E2E tests
make test-e2e-ui      # Interactive E2E test UI
make test-e2e-debug   # Debug E2E tests
```

### Using Cargo Directly

```bash
# Build specific package
cargo build -p loom-core

# Build with features
cargo build --features "feature1,feature2"

# Test specific package
cargo test -p loom-server

# Run binary
cargo run -p loom-cli -- --help

# Generate documentation
cargo doc --open

# Check for issues without building
cargo check --all

# Profile build time
cargo build --timings

# Bench (if benchmarks exist)
cargo bench -p loom-core
```

### Web Development (crates/loom-web)

```bash
# Development server with hot reload
cd crates/loom-web
npm run dev

# Production build
npm run build

# Run E2E tests
npm run test:e2e

# E2E tests in UI mode
npm run test:e2e:ui

# Type checking
npm run check

# Lint web code
npm run lint
```

### Git Workflows

```bash
# Create feature branch
git checkout -b feature/my-feature

# Commit with loom-style message
git commit -m "feat(loom-core): add new feature"

# Run pre-commit checks before pushing
make check

# Push changes
git push origin feature/my-feature

# Create pull request via GitHub CLI
gh pr create --title "My Feature" --body "Description"
```

## Development Workflow Example

```bash
# 1. Clone and setup
git clone https://github.com/ghuntley/loom.git
cd loom

# Using Nix (recommended)
direnv allow
# or
nix develop

# 2. Make changes
# Edit files in your IDE

# 3. Test locally
make build          # Ensure it compiles
make test           # Run unit tests
cd crates/loom-web && npm run test:e2e  # E2E tests

# 4. Check code quality
make check          # Format + lint + build + test

# 5. If all passes, commit and push
git add .
git commit -m "feat(module): description"
git push origin feature/branch-name

# 6. Create pull request and wait for CI
```

## Next Steps

1. **Read the architecture**: See `COMPONENT_ARCHITECTURE.md`
2. **Check examples**: See `examples/` directory
3. **Review contributing**: See `CONTRIBUTING.md`
4. **API documentation**: See `API_QUICK_REFERENCE.md`

## Getting Help

- **GitHub Issues**: https://github.com/ghuntley/loom/issues
- **Discussions**: https://github.com/ghuntley/loom/discussions
- **Documentation**: See `.md` files in root directory
- **Slack/Discord**: Check GitHub for community links
