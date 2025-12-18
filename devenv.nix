{ pkgs, lib, config, inputs, ... }:

let
  # Import the loom-server Nix package definition
  loomServerPkg = (import ./nix/loom-server.nix { inherit pkgs; });
in
{
  # https://devenv.sh/basics/
  env.GREET = "devenv";

  # https://devenv.sh/packages/
  packages = [ pkgs.git ];

  # https://devenv.sh/languages/
  languages.rust.enable = true;

  # https://devenv.sh/processes/
  # processes.dev.exec = "${lib.getExe pkgs.watchexec} -n -- ls -la";

  # https://devenv.sh/services/
  # services.postgres.enable = true;

  # https://devenv.sh/scripts/
  scripts.hello.exec = ''
    echo hello from $GREET
  '';

  # https://devenv.sh/basics/
  enterShell = ''
    hello         # Run scripts directly
    git --version # Use packages
  '';

  # https://devenv.sh/tasks/
  # tasks = {
  #   "myproj:setup".exec = "mytool build";
  #   "devenv:enterShell".after = [ "myproj:setup" ];
  # };

  # https://devenv.sh/tests/
  enterTest = ''
    echo "Running tests"
    git --version | grep --color=auto "${pkgs.git.version}"
  '';

  # https://devenv.sh/git-hooks/
  # git-hooks.hooks.shellcheck.enable = true;

  # https://devenv.sh/containers/
  # Production container definition for loom-server
  # Build with: devenv container build loom-server
  containers.loom-server = {
    # Image metadata
    name = "loom-server";

    # Runtime contents: only the loom-server binary and its runtime dependencies
    # Multi-stage by design: Nix builds the binary, then only that closure is included
    packages = [ loomServerPkg ];

    # Container runtime configuration
    config = {
      # Security: run as non-root user
      User = "1000:1000";

      # Entrypoint: run the loom-server binary
      Cmd = [ "${loomServerPkg}/bin/loom-server" ];

      # Expose HTTP server port
      ExposedPorts = {
        "8080/tcp" = { };
      };

      # Environment variables with sensible defaults
      Env = [
        "RUST_LOG=info"
      ];

      # Optional: health check via curl to the health endpoint
      # Uncomment if loom-server implements /health endpoint
      # Healthcheck = {
      #   Test = [ "CMD" "curl" "-f" "http://127.0.0.1:8080/health" ];
      #   Interval = 30000000000;  # 30 seconds in nanoseconds
      #   Timeout = 3000000000;    # 3 seconds in nanoseconds
      #   Retries = 3;
      # };
    };
  };

  # See full reference at https://devenv.sh/reference/options/
}
