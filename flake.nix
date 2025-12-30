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
      overlay = import ./infra/pkgs { fenix = fenixPkgs; };
      toolsOverlay = import ./tools/pkgs;
      mkSystem = modules: nixpkgs.lib.nixosSystem {
        inherit system;
        modules = modules ++ [
          ({ config, pkgs, ... }: {
            nixpkgs.overlays = [ overlay toolsOverlay ];
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
          pkgs = nixpkgs.legacyPackages.${system}.extend overlay;
          pkgsWithTools = pkgs.extend toolsOverlay;
        in
        {
          inherit (pkgs) smtprelay loom-server loom-cli loom-cli-linux loom-web;
          inherit (pkgs) loom-cli-windows loom-cli-macos loom-cli-linux-aarch64 loom-cli-windows-aarch64;
          inherit (pkgs) loom-cli-binaries weaver-image;
          inherit (pkgsWithTools) license;
        };
    };
}
