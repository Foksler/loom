# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Custom packages overlay
# Takes fenix as parameter for cross-compilation support.
# Usage: import ./infra/pkgs { inherit fenix; }
{ fenix ? null }:

final: prev:
let
  # Cross-compiled CLI packages (require fenix)
  loom-cli-windows = if fenix != null then
    final.callPackage ./loom-cli-windows.nix { inherit fenix; }
  else null;
  loom-cli-macos = if fenix != null then
    final.callPackage ./loom-cli-macos.nix { inherit fenix; }
  else null;
  loom-cli-linux-aarch64 = if fenix != null then
    final.callPackage ./loom-cli-linux-aarch64.nix { inherit fenix; }
  else null;
  loom-cli-windows-aarch64 = if fenix != null then
    final.callPackage ./loom-cli-windows-aarch64.nix { inherit fenix; }
  else null;
in
{
  smtprelay = final.callPackage ./smtprelay.nix { };
  loom-server = final.callPackage ./loom-server.nix { };
  loom-cli = final.callPackage ./loom-cli.nix { };
  loom-cli-linux = final.callPackage ./loom-cli-linux.nix { };
  loom-web = final.callPackage ./loom-web.nix { };
  
  inherit loom-cli-windows loom-cli-macos loom-cli-linux-aarch64 loom-cli-windows-aarch64;
  
  loom-cli-binaries = final.callPackage ./loom-cli-binaries.nix {
    loom-cli-linux = final.loom-cli-linux;
    inherit loom-cli-windows loom-cli-macos loom-cli-linux-aarch64 loom-cli-windows-aarch64;
  };

  weaver-image = final.callPackage ./weaver-image.nix {
    loom-cli = final.loom-cli;
  };
}
