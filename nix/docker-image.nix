# Nix expression for building loom-server Docker image
# Uses nixpkgs.dockerTools for reproducible image builds
{ pkgs }:

let
  loomServer = (import ./loom-server.nix { inherit pkgs; });
in
pkgs.dockerTools.buildImage {
  name = "loom-server";
  tag = "latest";

  # Copy binary and runtime dependencies to image root
  copyToRoot = pkgs.buildEnv {
    name = "image-root";
    paths = [
      loomServer
      pkgs.cacert
    ];
    pathsToLink = [ "/bin" "/etc" ];
  };

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
    ];

    # Working directory
    WorkingDir = "/";
  };
}
