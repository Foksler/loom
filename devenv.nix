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

  # https://devenv.sh/packages/
  packages = [ 
    pkgs.age
    pkgs.btop
    pkgs.cargo-watch
    pkgs.cosign      # Container image signing tool
    pkgs.curl
    pkgs.docker
    pkgs.dprint      # Universal code formatter (replaces prettier + rustfmt)
    pkgs.git
    pkgs.jq
    pkgs.lazygit
    tools.license    # License header management tool
    pkgs.nixos-rebuild
    pkgs.nodejs_22   # Node.js for web tooling compatibility
    pkgs.pnpm_9
    pkgs.redis
    pkgs.skopeo
    pkgs.sops
    pkgs.ssh-to-age
  ];

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
    
    # Ensure loom-server flake package compiles
    loom-server-build = {
      enable = true;
      name = "Build loom-server";
      entry = "${pkgs.writeShellScript "loom-server-build" ''
        echo "🔨 Building loom-server..."
        export NIXPKGS_ALLOW_UNFREE=1
        if ! nix build .#loom-server --no-link --impure 2>&1; then
          echo "❌ BLOCKED: loom-server failed to compile!"
          echo "Fix the build errors before committing."
          exit 1
        fi
        echo "✅ loom-server builds successfully"
      ''}";
      pass_filenames = false;
      always_run = true;
    };
    
    # Ensure loom-web flake package compiles
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
      always_run = true;
    };
  };

  # See full reference at https://devenv.sh/reference/options/
}
