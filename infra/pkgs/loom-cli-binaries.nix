# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Creates a directory structure with CLI binaries for distribution.
# The server serves these at /bin/{platform} for self-update functionality.
#
# Supported platforms:
# - linux-x86_64: Native Linux build (via loom-cli-linux)
# - windows-x86_64: Cross-compiled Windows build (via loom-cli-windows)
#
# Note: Both platform packages must be passed explicitly as they may require
# special build configurations (e.g., fenix for Windows cross-compilation).

{ lib
, stdenv
, loom-cli-linux
, loom-cli-windows ? null
}:

stdenv.mkDerivation {
  pname = "loom-cli-binaries";
  version = loom-cli-linux.version;

  dontUnpack = true;

  installPhase = ''
    mkdir -p $out
    # Copy CLI binaries with platform names expected by the update system
    # Platform naming: {os}-{arch} (e.g., linux-x86_64, windows-x86_64)
    
    # Linux x86_64
    cp ${loom-cli-linux}/bin/loom-linux-x86_64 $out/linux-x86_64
    
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
