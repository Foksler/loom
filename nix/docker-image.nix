# Nix expression for building loom-server Docker image with loom-web
# Uses nixpkgs.dockerTools for reproducible image builds
{ pkgs }:

let
  loomServer = (import ./loom-server.nix { inherit pkgs; });
  
  # Build loom-web using pnpm
  loomWeb = pkgs.buildNpmPackage {
    pname = "loom-web";
    version = "0.1.0";
    src = ../web/loom-web;
    
    npmDepsHash = "sha256-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA="; # TODO: Update after first build
    
    buildPhase = ''
      pnpm build
    '';
    
    installPhase = ''
      mkdir -p $out
      cp -r build/* $out/
    '';
  };
in
pkgs.dockerTools.buildImage {
  name = "loom-server";
  tag = "latest";

  # Copy binary, web assets, and runtime dependencies to image root
  copyToRoot = pkgs.buildEnv {
    name = "image-root";
    paths = [
      loomServer
      pkgs.cacert
    ];
    pathsToLink = [ "/bin" "/etc" ];
  };

  # Additional contents: web assets
  extraCommands = ''
    mkdir -p var/www/loom-web
    cp -r ${loomWeb}/* var/www/loom-web/
  '';

  # Container configuration
  config = {
    # Run as non-root user
    User = "1000:1000";

    # Default command: run loom-server
    Cmd = [ "${loomServer}/bin/loom-server" ];

    # Exposed ports
    ExposedPorts = {
      "8080/tcp" = { };
    };

    # Environment variables
    Env = [
      "RUST_LOG=info"
      "PATH=/usr/bin:/bin"
      "LOOM_SERVER_WEB_DIR=/var/www/loom-web"
    ];

    # Working directory
    WorkingDir = "/";
  };
}
