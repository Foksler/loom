# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

# Custom packages overlay
final: prev: {
  smtprelay = final.callPackage ./smtprelay.nix { };
}
