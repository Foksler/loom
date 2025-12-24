# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Creates a directory structure with CLI binaries for distribution.
# The server serves these at /bin/{platform} for self-update functionality.

{ lib
, stdenv
, loom-cli
}:

stdenv.mkDerivation {
  pname = "loom-cli-binaries";
  version = loom-cli.version;

  dontUnpack = true;

  installPhase = ''
    mkdir -p $out
    # Copy the CLI binary with the platform name expected by the update system
    # Platform naming: {os}-{arch} (e.g., linux-x86_64, macos-aarch64)
    cp ${loom-cli}/bin/loom $out/linux-x86_64
  '';

  meta = with lib; {
    description = "Loom CLI binaries packaged for server distribution";
    license = licenses.unfree;
  };
}
