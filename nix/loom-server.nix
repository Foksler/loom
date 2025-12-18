# Nix derivation for loom-server binary
# Produces a reproducible, optimized binary for container deployment
{ pkgs }:

let
  rustPlatform = pkgs.rustPlatform;
in
rustPlatform.buildRustPackage {
  pname = "loom-server";
  version = "0.1.0"; # Keep in sync with Cargo.toml workspace.version

  # Build from the whole workspace so Cargo can resolve members.
  src = pkgs.lib.cleanSource ./..;

  cargoLock.lockFile = ../Cargo.lock;

  # Only build the server crate to save time.
  cargoBuildFlags = [ "--package" "loom-server" "--locked" ];

  # Disable tests here; rely on CI and Make test target instead.
  doCheck = false;

  # If the server has extra native dependencies, add them here.
  buildInputs = [ ];

  # Strip the binary to reduce size.
  dontStrip = false;

  # Post-build phases can add optimizations if needed.
  postInstall = ''
    echo "loom-server binary built successfully"
    ls -lh $out/bin/loom-server
  '';

  meta = {
    description = "Loom thread persistence server";
    homepage = "https://github.com/ghuntley/loom";
    license = pkgs.lib.licenses.unfree; # Adjust to actual license
  };
}
