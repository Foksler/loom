# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Creates a directory structure with CLI binaries for distribution.
# The server serves these at /bin/{platform} for self-update functionality.
#
# Supported platforms:
# - linux-x86_64: Native Linux build
# - windows-x86_64: Cross-compiled Windows build (via loom-cli-windows package)
#
# Note: loom-cli-windows must be passed explicitly when Windows support is needed,
# as it requires fenix which is only available through the flake.

{ lib
, stdenv
, loom-cli
, loom-cli-windows ? null
}:

stdenv.mkDerivation {
  pname = "loom-cli-binaries";
  version = loom-cli.version;

  dontUnpack = true;

  installPhase = ''
    mkdir -p $out
    # Copy the CLI binary with the platform name expected by the update system
    # Platform naming: {os}-{arch} (e.g., linux-x86_64, macos-aarch64)
    
    # Linux x86_64 (native)
    cp ${loom-cli}/bin/loom $out/linux-x86_64
    
    # Windows x86_64 (cross-compiled via fenix)
    ${lib.optionalString (loom-cli-windows != null) ''
      cp ${loom-cli-windows}/bin/loom-windows-x86_64.exe $out/windows-x86_64.exe
    ''}
  '';

  meta = with lib; {
    description = "Loom CLI binaries packaged for server distribution";
    license = licenses.unfree;
  };
}
