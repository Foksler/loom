# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Custom packages overlay
# Note: loom-cli-windows and loom-cli-binaries are defined in flake.nix
# as they require fenix for cross-compilation support.
final: prev: {
  smtprelay = final.callPackage ./smtprelay.nix { };
  loom-server = final.callPackage ./loom-server.nix { };
  loom-cli = final.callPackage ./loom-cli.nix { };
  loom-cli-linux = final.callPackage ./loom-cli-linux.nix { };
  loom-web = final.callPackage ./loom-web.nix { };
}
