# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{
  description = "NixOS machine configurations";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    nixos-vscode-server = {
      url = "github:nix-community/nixos-vscode-server";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs = { self, nixpkgs, nixos-vscode-server, sops-nix }:
    let
      overlay = import ./infra/pkgs;
      toolsOverlay = import ./tools/pkgs;
      mkSystem = modules: nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
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
          pkgs = nixpkgs.legacyPackages.x86_64-linux.extend overlay;
          pkgsWithTools = pkgs.extend toolsOverlay;
        in
        {
          inherit (pkgs) smtprelay loom-server loom-web;
          inherit (pkgsWithTools) license;
        };
    };
}
