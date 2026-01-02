# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ pkgs, lib, config, inputs, ... }:

let
  # Import our tools overlay to get access to custom packages
  tools = pkgs.extend (import ./tools/pkgs);
in
{

  # https://devenv.sh/basics/
  env.GREET = "devenv";
  
  # Faster git operations for cargo
  env.CARGO_NET_GIT_FETCH_WITH_CLI = "true";
  
  # Library path for native dependencies (zlib, openssl, etc.)
  # Required for build scripts that link dynamically to C libraries
  env.LD_LIBRARY_PATH = lib.makeLibraryPath [
    pkgs.zlib
    pkgs.openssl
  ];

  

  # https://devenv.sh/packages/
  packages = [ 
    pkgs.age
    pkgs.btop
    pkgs.clang        # For mold linker wrapper
    pkgs.zlib         # Required by libz-sys (git2, etc.)
    pkgs.gettext      # For msgfmt (i18n .po → .mo compilation)
    pkgs.cargo-watch
    pkgs.cosign      # Container image signing tool
    pkgs.curl
    pkgs.docker
    pkgs.dprint      # Universal code formatter (replaces prettier + rustfmt)
    pkgs.git
    pkgs.jq
    pkgs.lazygit
    tools.license    # License header management tool
    pkgs.mold        # Fast linker for Rust (see .cargo/config.toml)
    pkgs.nixos-rebuild
    pkgs.nodejs_22   # Node.js for web tooling compatibility
    pkgs.pnpm_9
    pkgs.redis
    pkgs.skopeo
    pkgs.sops
    pkgs.ssh-to-age
  ];
  
  # Shell aliases and scripts for cargo2nix workflow
  scripts.cargo2nix-update.exec = ''
    echo "🔄 Regenerating Cargo.nix from Cargo.lock..."
    nix run github:cargo2nix/cargo2nix/release-0.12
    echo "✅ Cargo.nix updated. Don't forget to commit it!"
  '';

  # https://devenv.sh/languages/
  languages.rust.enable = true;
  languages.rust.components = [ "rustc" "cargo" "clippy" "rustfmt" ];
  languages.typescript.enable = true;
  languages.javascript.pnpm.enable = true;

  # https://devenv.sh/processes/
  
  # https://devenv.sh/tasks/

  # https://devenv.sh/tests/

  # Shell aliases and helper functions
  enterShell = ''
  '';

  # https://devenv.sh/git-hooks/
  git-hooks.hooks = {
    # Code quality
    shellcheck.enable = true;
    
    # Block backup files that might contain secrets
    block-backup-files = {
      enable = true;
      name = "Block backup files";
      entry = "${pkgs.writeShellScript "block-backup-files" ''
        if git diff --cached --name-only | grep -E '\.(backup|bak|tmp|temp|orig|copy|swp)$|~$'; then
          echo "❌ BLOCKED: Backup/temp files detected in commit!"
          echo "These files might contain secrets and should not be committed:"
          git diff --cached --name-only | grep -E '\.(backup|bak|tmp|temp|orig|copy|swp)$|~$'
          echo ""
          echo "Please review these files and add them to .gitignore if needed."
          exit 1
        fi
      ''}";
      types = [ "text" ];
    };
    
    # Block secrets directory backup files specifically
    block-secrets-backup = {
      enable = true;
      name = "Block secrets backup files";
      entry = "${pkgs.writeShellScript "block-secrets-backup" ''
        if git diff --cached --name-only | grep -E 'secrets/.*\.(backup|bak)$'; then
          echo "🚨 CRITICAL: Encrypted secrets backup file detected!"
          echo "Files blocked:"
          git diff --cached --name-only | grep -E 'secrets/.*\.(backup|bak)$'
          echo ""
          echo "These files contain encrypted secrets and MUST NOT be committed to git."
          echo "Use 'git reset HEAD <file>' to unstage them."
          exit 1
        fi
      ''}";
      types = [ "text" ];
    };
    
    # Fast Rust workspace check using cargo2nix (nix builds with caching)
    # Uses granular per-crate nix builds for reproducibility and better caching.
    # Individual crates are cached in the nix store, so only changed crates rebuild.
    rust-workspace-nix = {
      enable = true;
      name = "Rust workspace check (nix/cargo2nix)";
      entry = "${pkgs.writeShellScript "rust-workspace-nix" ''
        echo "🔨 Building Rust workspace with nix (cargo2nix)..."
        
        # Build the main binaries using cargo2nix
        # This provides reproducible builds with per-crate caching
        if ! nix build .#loom-cli-c2n .#loom-server-c2n --no-link 2>&1; then
          echo "❌ BLOCKED: Rust workspace failed to compile!"
          echo "Fix the build errors before committing."
          exit 1
        fi
        
        echo "✅ Rust workspace compiles successfully (nix)"
      ''}";
      pass_filenames = false;
      # Only run when Rust-related files change
      always_run = false;
      types = [ "rust" ];
    };
    
    # Optional: Fast cargo check for quick iteration (can be enabled alongside nix builds)
    # Disabled by default since nix builds provide better reproducibility
    rust-workspace-cargo = {
      enable = false;
      name = "Rust workspace check (cargo, fast)";
      entry = "${pkgs.writeShellScript "rust-workspace-check" ''
        echo "🔨 Checking Rust workspace with cargo (incremental)..."
        
        if ! cargo check --workspace --bins 2>&1; then
          echo "❌ BLOCKED: Rust workspace failed to compile!"
          echo "Fix the build errors before committing."
          exit 1
        fi
        
        echo "✅ Rust workspace compiles successfully"
      ''}";
      pass_filenames = false;
      always_run = false;
      types = [ "rust" ];
    };
    
    # Run clippy via nix for linting
    # Uses the cargo2nix workspace shell for consistent toolchain
    clippy = {
      enable = true;
      name = "Clippy lint check";
      entry = "${pkgs.writeShellScript "clippy-check" ''
        echo "🔍 Running clippy..."
        
        # Use cargo-clippy directly (nix-provided) to avoid rustup conflicts
        if ! cargo-clippy --all-targets --all-features -- -D warnings 2>&1; then
          echo "❌ BLOCKED: Clippy found warnings/errors!"
          echo "Fix the clippy issues before committing."
          exit 1
        fi
        
        echo "✅ Clippy passed"
      ''}";
      pass_filenames = false;
      always_run = false;
      types = [ "rust" ];
    };
    
    # Ensure loom-web flake package compiles (fast - Node.js cached build)
    loom-web-build = {
      enable = true;
      name = "Build loom-web";
      entry = "${pkgs.writeShellScript "loom-web-build" ''
        echo "🔨 Building loom-web..."
        export NIXPKGS_ALLOW_UNFREE=1
        if ! nix build .#loom-web --no-link --impure 2>&1; then
          echo "❌ BLOCKED: loom-web failed to compile!"
          echo "Fix the build errors before committing."
          exit 1
        fi
        echo "✅ loom-web builds successfully"
      ''}";
      pass_filenames = false;
      # Only run when web-related files change
      always_run = false;
      files = "^web/";
    };
    
    # Build loom-cli using cargo2nix (reproducible, cached per-crate)
    loom-cli-build = {
      enable = false;  # Disabled - covered by rust-workspace-nix
      name = "Build loom-cli (nix)";
      entry = "${pkgs.writeShellScript "loom-cli-build" ''
        echo "🔨 Building loom-cli with nix..."
        
        if ! nix build .#loom-cli-c2n --no-link 2>&1; then
          echo "❌ BLOCKED: loom-cli failed to compile!"
          echo "Fix the build errors before committing."
          exit 1
        fi
        
        echo "✅ loom-cli builds successfully (nix)"
      ''}";
      pass_filenames = false;
      always_run = false;
      types = [ "rust" ];
    };
    
    # Build loom-server using cargo2nix (reproducible, cached per-crate)
    loom-server-build = {
      enable = false;  # Disabled - covered by rust-workspace-nix
      name = "Build loom-server (nix)";
      entry = "${pkgs.writeShellScript "loom-server-build" ''
        echo "🔨 Building loom-server with nix..."
        
        if ! nix build .#loom-server-c2n --no-link 2>&1; then
          echo "❌ BLOCKED: loom-server failed to compile!"
          echo "Fix the build errors before committing."
          exit 1
        fi
        
        echo "✅ loom-server builds successfully (nix)"
      ''}";
      pass_filenames = false;
      always_run = false;
      types = [ "rust" ];
    };
  };

  # See full reference at https://devenv.sh/reference/options/
}
