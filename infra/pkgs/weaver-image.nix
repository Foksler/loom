# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Nix expression for building loom-weaver Docker image
# Uses nixpkgs.dockerTools for reproducible image builds
#
# The weaver image provides an ephemeral environment for running loom REPL
# sessions in isolated Kubernetes pods. It includes:
# - loom CLI binary
# - git for repository cloning
# - Development tools (gh, btop, tmux, jq)
# - Entrypoint script for repo cloning and REPL startup

{ lib
, dockerTools
, buildEnv
, writeShellScriptBin
, loom-cli
, cacert
, git
, curl
, gh
, btop
, tmux
, jq
, coreutils
, bashInteractive
}:

let
  # Entrypoint script for weaver pods
  entrypoint = writeShellScriptBin "entrypoint" ''
    #!/bin/bash
    # Weaver pod entrypoint script
    # Clones a git repository if specified, then starts loom REPL

    set -e

    WORKSPACE="/workspace"

    # Clone repository if LOOM_REPO is set
    if [ -n "$LOOM_REPO" ]; then
      echo "Cloning $LOOM_REPO..."
      
      if [ -n "$LOOM_BRANCH" ]; then
        ${git}/bin/git clone --branch "$LOOM_BRANCH" --single-branch "$LOOM_REPO" "$WORKSPACE"
      else
        ${git}/bin/git clone "$LOOM_REPO" "$WORKSPACE"
      fi
      
      cd "$WORKSPACE"
      echo "Cloning complete."
      echo ""
    else
      mkdir -p "$WORKSPACE"
      cd "$WORKSPACE"
    fi

    # Start loom REPL
    exec ${loom-cli}/bin/loom
  '';

  # Create passwd and group files for the loom user
  passwdFile = builtins.toFile "passwd" ''
    root:x:0:0:root:/root:/bin/bash
    loom:x:1000:1000:loom:/home/loom:/bin/bash
  '';

  groupFile = builtins.toFile "group" ''
    root:x:0:
    loom:x:1000:
  '';
in
dockerTools.buildImage {
  name = "loom-weaver";
  tag = "latest";

  # Copy binaries and runtime dependencies to image root
  copyToRoot = buildEnv {
    name = "weaver-root";
    paths = [
      loom-cli
      entrypoint
      cacert
      git
      curl
      gh
      btop
      tmux
      jq
      coreutils
      bashInteractive
    ];
    pathsToLink = [ "/bin" "/etc" "/share" ];
  };

  # Additional configuration
  extraCommands = ''
    # Create directory structure
    mkdir -p home/loom
    mkdir -p workspace
    mkdir -p tmp
    mkdir -p etc

    # Create passwd and group files
    cp ${passwdFile} etc/passwd
    cp ${groupFile} etc/group

    # Set permissions
    chmod 1777 tmp
    chmod 755 home/loom
    chmod 755 workspace
  '';

  # Container configuration
  config = {
    # Run as non-root user
    User = "1000:1000";

    # Entrypoint
    Entrypoint = [ "${entrypoint}/bin/entrypoint" ];

    # Exposed ports (none needed for weaver)
    ExposedPorts = { };

    # Environment variables
    Env = [
      "RUST_LOG=info"
      "PATH=/bin"
      "HOME=/home/loom"
      "USER=loom"
      "TERM=xterm-256color"
      "SSL_CERT_FILE=/etc/ssl/certs/ca-bundle.crt"
    ];

    # Working directory
    WorkingDir = "/workspace";

    # Labels
    Labels = {
      "org.opencontainers.image.title" = "Loom Weaver";
      "org.opencontainers.image.description" = "Ephemeral environment for Loom REPL sessions";
      "org.opencontainers.image.source" = "https://github.com/ghuntley/loom";
    };
  };
}
