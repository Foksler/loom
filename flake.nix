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
    cargo2nix = {
      url = "github:cargo2nix/cargo2nix/release-0.12";
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

  outputs = { self, nixpkgs, fenix, cargo2nix, nixos-vscode-server, sops-nix }:
    let
      system = "x86_64-linux";
      fenixPkgs = fenix.packages.${system};
      # Overlay with cross-compilation support (for packages output)
      overlayWithCross = import ./infra/pkgs { fenix = fenixPkgs; };
      # Overlay without cross-compilation (for NixOS system - faster builds)
      overlayNoCross = import ./infra/pkgs { fenix = null; };
      toolsOverlay = import ./tools/pkgs;
      
      # cargo2nix overlay for granular crate builds
      cargo2nixOverlay = cargo2nix.overlays.default;
      
      mkSystem = modules: nixpkgs.lib.nixosSystem {
        inherit system;
        modules = modules ++ [
          ({ config, pkgs, ... }: {
            nixpkgs.overlays = [ overlayNoCross toolsOverlay ];
          })
        ];
      };
      
      # Create package set with cargo2nix for granular crate builds
      pkgsWithCargo2nix = import nixpkgs {
        inherit system;
        config = { allowUnfree = true; };
        overlays = [ cargo2nixOverlay ];
      };
      
      # Build the rust package set from Cargo.nix
      rustPkgs = pkgsWithCargo2nix.rustBuilder.makePackageSet {
        rustVersion = "1.83.0";
        packageFun = import ./Cargo.nix;
        workspaceSrc = ./.;
        packageOverrides = pkgs: pkgs.rustBuilder.overrides.all ++ [
          # Add custom overrides for crates that need native dependencies
          (pkgs.rustBuilder.rustLib.makeOverride {
            name = "openssl-sys";
            overrideAttrs = drv: {
              nativeBuildInputs = (drv.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
              buildInputs = (drv.buildInputs or []) ++ [ pkgs.openssl ];
            };
          })
          (pkgs.rustBuilder.rustLib.makeOverride {
            name = "libsqlite3-sys";
            overrideAttrs = drv: {
              nativeBuildInputs = (drv.nativeBuildInputs or []) ++ [ pkgs.pkg-config ];
              buildInputs = (drv.buildInputs or []) ++ [ pkgs.sqlite ];
            };
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
          
          # cargo2nix-based granular crate builds
          # All workspace crates exposed individually
          loom-cli-c2n = (rustPkgs.workspace.loom-cli {});
          loom-server-c2n = (rustPkgs.workspace.loom-server {});
          loom-cli-acp-c2n = (rustPkgs.workspace.loom-cli-acp {});
          loom-cli-auto-commit-c2n = (rustPkgs.workspace.loom-cli-auto-commit {});
          loom-cli-config-c2n = (rustPkgs.workspace.loom-cli-config {});
          loom-cli-credentials-c2n = (rustPkgs.workspace.loom-cli-credentials {});
          loom-cli-git-c2n = (rustPkgs.workspace.loom-cli-git {});
          loom-cli-tools-c2n = (rustPkgs.workspace.loom-cli-tools {});
          loom-common-config-c2n = (rustPkgs.workspace.loom-common-config {});
          loom-common-core-c2n = (rustPkgs.workspace.loom-common-core {});
          loom-common-http-c2n = (rustPkgs.workspace.loom-common-http {});
          loom-common-i18n-c2n = (rustPkgs.workspace.loom-common-i18n {});
          loom-common-secret-c2n = (rustPkgs.workspace.loom-common-secret {});
          loom-common-thread-c2n = (rustPkgs.workspace.loom-common-thread {});
          loom-common-version-c2n = (rustPkgs.workspace.loom-common-version {});
          loom-server-api-c2n = (rustPkgs.workspace.loom-server-api {});
          loom-server-auth-c2n = (rustPkgs.workspace.loom-server-auth {});
          loom-server-auth-devicecode-c2n = (rustPkgs.workspace.loom-server-auth-devicecode {});
          loom-server-auth-github-c2n = (rustPkgs.workspace.loom-server-auth-github {});
          loom-server-auth-google-c2n = (rustPkgs.workspace.loom-server-auth-google {});
          loom-server-auth-magiclink-c2n = (rustPkgs.workspace.loom-server-auth-magiclink {});
          loom-server-auth-okta-c2n = (rustPkgs.workspace.loom-server-auth-okta {});
          loom-server-db-c2n = (rustPkgs.workspace.loom-server-db {});
          loom-server-geoip-c2n = (rustPkgs.workspace.loom-server-geoip {});
          loom-server-github-app-c2n = (rustPkgs.workspace.loom-server-github-app {});
          loom-server-google-cse-c2n = (rustPkgs.workspace.loom-server-google-cse {});
          loom-server-jobs-c2n = (rustPkgs.workspace.loom-server-jobs {});
          loom-server-k8s-c2n = (rustPkgs.workspace.loom-server-k8s {});
          loom-server-llm-anthropic-c2n = (rustPkgs.workspace.loom-server-llm-anthropic {});
          loom-server-llm-openai-c2n = (rustPkgs.workspace.loom-server-llm-openai {});
          loom-server-llm-proxy-c2n = (rustPkgs.workspace.loom-server-llm-proxy {});
          loom-server-llm-service-c2n = (rustPkgs.workspace.loom-server-llm-service {});
          loom-server-llm-vertex-c2n = (rustPkgs.workspace.loom-server-llm-vertex {});
          loom-server-scm-c2n = (rustPkgs.workspace.loom-server-scm {});
          loom-server-scm-mirror-c2n = (rustPkgs.workspace.loom-server-scm-mirror {});
          loom-server-smtp-c2n = (rustPkgs.workspace.loom-server-smtp {});
          loom-server-weaver-c2n = (rustPkgs.workspace.loom-server-weaver {});
          
          # Combined workspace build for pre-commit validation
          loom-workspace-c2n = pkgsWithCargo2nix.symlinkJoin {
            name = "loom-workspace-c2n";
            paths = [
              (rustPkgs.workspace.loom-cli {})
              (rustPkgs.workspace.loom-server {})
            ];
          };
        };
      
      # Expose cargo2nix overlay for devenv integration
      overlays.cargo2nix = cargo2nixOverlay;
      
      # Expose rustPkgs for pre-commit hooks
      lib.rustPkgs = rustPkgs;
    };
}
