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
          
          # Platform-specific CLI builds
          loom-cli-linux = pkgs.loom-cli-linux;
          loom-cli-windows = pkgs.callPackage ./infra/pkgs/loom-cli-windows.nix {
            fenix = fenixPkgs;
          };
          loom-cli-macos = pkgs.callPackage ./infra/pkgs/loom-cli-macos.nix {
            fenix = fenixPkgs;
          };
          loom-cli-linux-aarch64 = pkgs.callPackage ./infra/pkgs/loom-cli-linux-aarch64.nix {
            fenix = fenixPkgs;
          };
        in
        {
          inherit (pkgs) smtprelay loom-server loom-cli loom-cli-linux loom-web;
          inherit (pkgsWithTools) license;
          inherit loom-cli-windows loom-cli-macos loom-cli-linux-aarch64;
          
          # Combined binaries for server distribution
          loom-cli-binaries = pkgs.callPackage ./infra/pkgs/loom-cli-binaries.nix {
            inherit loom-cli-linux loom-cli-windows loom-cli-macos loom-cli-linux-aarch64;
          };
        };
    };
}
