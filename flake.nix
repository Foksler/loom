# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{
  description = "NixOS machine configurations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    nixos-vscode-server = {
      url = "github:nix-community/nixos-vscode-server";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, fenix, nixos-vscode-server, sops-nix }:
    let
      system = "x86_64-linux";
      fenixPkgs = fenix.packages.${system};
      # Overlay with cross-compilation support (for packages output)
      overlayWithCross = import ./infra/pkgs { fenix = fenixPkgs; };
      # Overlay without cross-compilation (for NixOS system - faster builds)
      overlayNoCross = import ./infra/pkgs { fenix = null; };
      toolsOverlay = import ./tools/pkgs;
      mkSystem = modules: nixpkgs.lib.nixosSystem {
        inherit system;
        modules = modules ++ [
          ({ config, pkgs, ... }: {
            nixpkgs.overlays = [ overlayNoCross toolsOverlay ];
          })
        ];
      };
    in
    {
      nixosConfigurations = {
        virtualMachine = mkSystem [
          ./infra/machines/loom.nix
          nixos-vscode-server.nixosModules.default
          sops-nix.nixosModules.sops
        ];
      };

      packages.x86_64-linux = 
        let
          pkgs = import nixpkgs {
            inherit system;
            config = { allowUnfree = true; };
            overlays = [ overlayWithCross ];
          };
          pkgsWithTools = pkgs.extend toolsOverlay;
        in
        {
          inherit (pkgs) smtprelay loom-server loom-cli loom-cli-linux loom-web;
          inherit (pkgs) loom-cli-windows loom-cli-macos loom-cli-linux-aarch64 loom-cli-windows-aarch64;
          inherit (pkgs) loom-weaver-binaries loom-server-binaries weaver-image loom-server-image;
          inherit (pkgsWithTools) license;
        };
    };
}
