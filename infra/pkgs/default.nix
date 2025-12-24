# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Custom packages overlay
final: prev: {
  smtprelay = final.callPackage ./smtprelay.nix { };
  loom-server = final.callPackage ./loom-server.nix { };
  loom-cli = final.callPackage ./loom-cli.nix { };
  loom-cli-binaries = final.callPackage ./loom-cli-binaries.nix { };
  loom-web = final.callPackage ./loom-web.nix { };
}
