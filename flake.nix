{
  description = "Loom - AI-powered coding assistant";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs = { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (system:
      let
        pkgs = (import nixpkgs {
          inherit system;
          config = {
            allowUnfree = true;
          };
        });
      in
      {
        packages = {
          # Rust binary
          loom-server = (import ./nix/loom-server.nix { inherit pkgs; });

          # Docker image
          loom-server-image = (import ./nix/docker-image.nix { inherit pkgs; });

          # Default package
          default = self.packages.${system}.loom-server;
        };

        apps = {
          # Build and load Docker image
          docker-build = {
            type = "app";
            program = toString (pkgs.writeShellScript "docker-build" ''
              set -e
              echo "Building loom-server Docker image..."
              nix build .#loom-server-image
              echo ""
              echo "✓ Docker image built successfully"
              echo "  Output: ./result (OCI/Docker image tarball)"
              echo ""
              echo "To load into Docker:"
              echo "  docker load < ./result"
              echo ""
              echo "To run:"
              echo "  docker run --rm -p 8080:8080 loom-server:latest"
            '');
          };

          # Build, load, and run Docker image
          docker-run = {
            type = "app";
            program = toString (pkgs.writeShellScript "docker-run" ''
              set -e
              echo "Building loom-server Docker image..."
              nix build .#loom-server-image

              echo "Loading image into Docker..."
              docker load < ./result

              echo ""
              echo "✓ Starting loom-server container (Ctrl+C to stop)"
              echo ""
              docker run --rm -p 8080:8080 loom-server:latest
            '');
          };
        };

        devShells.default = pkgs.mkShell {
          buildInputs = with pkgs; [
            rustup
            cargo
            clippy
            rustfmt
            pkg-config
            openssl
            git
            nix-prefetch-git
            docker
          ];
        };
      }
    );
}
