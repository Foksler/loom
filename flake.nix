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
          system = "x86_64-linux";
          pkgs = nixpkgs.legacyPackages.${system}.extend overlay;
          pkgsWithTools = pkgs.extend toolsOverlay;
          fenixPkgs = fenix.packages.${system};
        in
        {
          inherit (pkgs) smtprelay loom-server loom-cli loom-cli-binaries loom-web;
          inherit (pkgsWithTools) license;
          
          # Windows cross-compilation uses fenix for Rust with Windows target
          loom-cli-windows = pkgs.callPackage ./infra/pkgs/loom-cli-windows.nix {
            fenix = fenixPkgs;
          };
        };
    };
}
