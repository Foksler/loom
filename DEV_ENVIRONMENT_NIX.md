# Nix Development Environment Setup

Complete guide for setting up Loom development environment using Nix with devenv and direnv.

## Why Nix?

- **Reproducible**: Exact same development environment across all machines
- **Isolated**: No conflicts with system-wide packages
- **Declarative**: All dependencies defined in version-controlled files
- **Cross-platform**: Works on Linux and macOS
- **Automatic**: Auto-loads environment when entering directory

## Quick Start

### Prerequisites

- **Nix 2.10+** with flakes support
- **direnv** (recommended, optional)

### Installation

#### 1. Install Nix (if not already installed)

**Linux/macOS**:
```bash
# Multi-user installation (recommended)
sh <(curl -L https://nixos.org/nix/install) --daemon

# Single-user (simpler)
sh <(curl -L https://nixos.org/nix/install) --no-daemon

# Restart shell
exec $SHELL
```

**Verify installation**:
```bash
nix --version
# Should output: nix (Nix) 2.10.0 or later
```

#### 2. Enable Nix Flakes (if not enabled)

```bash
# Create config directory
mkdir -p ~/.config/nix

# Enable experimental features
echo "experimental-features = nix-command flakes" >> ~/.config/nix/nix.conf

# Verify
nix flake --version
```

#### 3. Install direnv (Optional but Recommended)

**macOS**:
```bash
brew install direnv
```

**Linux (Ubuntu/Debian)**:
```bash
sudo apt-get install direnv
```

**Linux (Fedora)**:
```bash
sudo dnf install direnv
```

**Other (Nix)**:
```bash
nix profile install nixpkgs#direnv
```

**Add direnv hook to shell** (one-time):

For Bash (add to `~/.bashrc`):
```bash
eval "$(direnv hook bash)"
```

For Zsh (add to `~/.zshrc`):
```bash
eval "$(direnv hook zsh)"
```

For Fish (add to `~/.config/fish/config.fish`):
```fish
direnv hook fish | source
```

**Restart shell**:
```bash
exec $SHELL
```

## Using Loom with Nix

### Method 1: With direnv (Recommended - Fully Automatic)

```bash
# Clone and enter directory
git clone https://github.com/ghuntley/loom.git
cd loom

# Allow direnv to load the environment
direnv allow

# Environment automatically loads whenever you enter the directory!
# Run any make command:
make build
make test
make check
```

**Benefits**:
- Automatic environment loading/unloading
- Development dependencies only active in directory
- Clear `.envrc` file shows what's loaded

### Method 2: Without direnv (Manual)

```bash
# Clone repository
git clone https://github.com/ghuntley/loom.git
cd loom

# Update flake lock file
nix flake update

# Enter development environment
nix develop

# Now in shell with all dependencies
make build
make test

# Exit environment
exit
```

## Environment Contents

The `devenv.nix` and `flake.nix` provide:

### Languages & Runtimes
- **Rust** (stable toolchain)
- **Cargo** (Rust package manager)
- **Node.js 18+** (for web development)
- **npm/yarn** (package managers)

### Tools
- **git** (version control)
- **curl** (HTTP client)
- **pkg-config** (library discovery)
- **just** (command runner, optional)

### Build Tools
- **make** (GNU Make)
- **cargo-clippy** (linter)
- **cargo-fmt** (formatter)
- **rustfmt** (code formatter)

### Optional Development Tools
- **cargo-watch** (rerun on file changes)
- **cargo-expand** (expand macros)
- **cargo-edit** (manage dependencies)
- **cargo-outdated** (check updates)

## File Structure

```
loom/
├── flake.nix           # Main Nix flake definition
├── flake.lock          # Locked dependency versions (commit this!)
├── devenv.nix          # Development environment setup
├── devenv.lock         # devenv lock file (commit this!)
├── .envrc              # direnv configuration
├── Makefile            # Development commands
└── ...
```

## Common Tasks

### Update Dependencies

```bash
# Update Nix flake dependencies
nix flake update

# Update devenv
devenv update
```

### Check What's Installed

```bash
nix develop --impure -c env | grep -E '^(RUST|NODE|CARGO)'

# Or list all available commands
nix develop --impure -c which -a cargo
```

### Run Specific Tools

```bash
# Without entering shell
nix develop --impure -c cargo --version
nix develop --impure -c node --version
nix develop --impure -c make build

# Or enter shell once
nix develop
cargo --version
node --version
make build
exit
```

### Clean Cache

```bash
# Clear Nix store of unused derivations
nix-collect-garbage

# More aggressive cleanup (removes everything not in use)
nix-collect-garbage -d

# Remove old profiles
rm /nix/var/nix/profiles/per-user/$USER/profile-*-link
```

## Troubleshooting

### Issue: "experimental-features" not recognized

**Solution**: Update Nix to latest version
```bash
nix upgrade-nix
# or
nix upgrade-nix --experimental-features 'nix-command flakes'
```

### Issue: direnv complains about `PATH`

**Solution**: Add to shell config:
```bash
# For bash/zsh: add to ~/.bashrc or ~/.zshrc
export DIRENV_LOG_FORMAT=""
eval "$(direnv hook bash)"  # or zsh for Zsh
```

### Issue: `nix flake update` hangs

**Solution**:
```bash
# Kill the process and try with timeout
timeout 120 nix flake update

# Or clear cache
rm flake.lock
nix flake update --commit-lock-file
```

### Issue: Out of disk space from `/nix`

**Solution**:
```bash
# Clean old derivations
nix-collect-garbage -d

# Clean everything (careful!)
nix-collect-garbage -d --delete-old

# Check size
du -sh /nix
```

### Issue: `which cargo` returns nothing in new shell

**Solution**: Ensure direnv is properly configured:
```bash
# Check if direnv is active
echo $DIRENV_FILE

# If empty, reload shell
exec $SHELL

# Or manually enter environment
nix develop
```

### Issue: Leptos build fails in Nix environment

**Solution**: Ensure Node.js is available:
```bash
nix develop
node --version  # Should show version
npm list leptos  # Check if Leptos installed

# If issues persist, try:
nix flake update
nix develop --impure
```

## Performance Tips

### Speed Up Builds

```bash
# Use all CPU cores
export NIX_BUILD_CORES=0

# Use more memory for builds (if available)
export NIX_BUILD_CORES=4

# Add to ~/.config/nix/nix.conf for persistence:
# build-cores = 0
# max-jobs = auto
```

### Cache Builds Across Machines

```bash
# Enable binary cache (in ~/.config/nix/nix.conf)
substituters = https://cache.nixos.org https://nix-community.cachix.org
trusted-public-keys = cache.nixos.org-1:6NCHdD59X431o0gWypQaGrRiv0gQCsxDMJMZ19v6Mbc= nix-community.cachix.org-1:mB9FSh9qf2QlZceNZSbVgNn8CO8/b5vgr7kfzpAGm7M=
```

## Advanced Configuration

### Custom Shell Hooks

Edit `devenv.nix` to add scripts that run on shell entry:

```nix
enterShell = ''
  echo "Welcome to Loom development!"
  echo "Rust: $(rustc --version)"
  echo "Node: $(node --version)"
  echo ""
  echo "Run 'make help' for available commands"
'';
```

### Adding More Tools

```nix
# In devenv.nix, add to packages:
packages = [
  pkgs.git
  pkgs.just
  pkgs.watchexec
  pkgs.ripgrep
];
```

### Custom Commands

```nix
# In devenv.nix:
scripts.dev.exec = ''
  cargo leptos watch
'';

# Then run:
# dev  # Automatically runs the script
```

## Documentation

- **Nix Manual**: https://nixos.org/manual/nix/stable/
- **devenv.sh**: https://devenv.sh/
- **direnv Manual**: https://direnv.net/
- **Flakes**: https://nixos.wiki/wiki/Flakes

## Next Steps

1. Set up environment:
   ```bash
   cd loom
   direnv allow  # or: nix develop
   ```

2. Build the project:
   ```bash
   make build
   ```

3. Run tests:
   ```bash
   make test
   ```

4. Start development:
   ```bash
   make dev  # or: cargo leptos watch
   ```

## Help

If you encounter issues:

1. Check the Nix documentation
2. Review `flake.nix` and `devenv.nix`
3. Try: `nix develop --impure` for more verbose output
4. Open a GitHub issue with your error

---

**Note**: Nix setup is optional. You can also use traditional installation methods as described in `DEV_ENVIRONMENT_SETUP.md`.
