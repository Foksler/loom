# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ config, lib, pkgs, modulesPath, ... }:

{
  imports = [
    (modulesPath + "/profiles/qemu-guest.nix")
    ../nixos-modules/base.nix
    ../nixos-modules/i18n.nix
    ../nixos-modules/known-hosts.nix
    ../nixos-modules/nix-settings.nix
    ../nixos-modules/pkgs.nix
    ../nixos-modules/secrets.nix
    ../nixos-modules/security-audit.nix
    ../nixos-modules/ssh.nix
    ../nixos-modules/sudo.nix
    ../nixos-modules/sysctl.nix
    ../nixos-modules/tailscale.nix
    ../nixos-modules/time.nix
    ../nixos-modules/user.nix
    ../nixos-modules/vscode-server.nix
    ../nixos-modules/nixos-auto-update.nix
    ../nixos-modules/loom-server.nix
    ../nixos-modules/loom-web.nix
  ];

  # Machine-specific configuration
  networking.hostName = "loom";

  # Networking
  networking.networkmanager.enable = false;
  networking.useDHCP = false;
  networking.interfaces.eth0.ipv4.addresses = [{
    address = "51.161.140.159";
    prefixLength = 32; # 255.255.255.255
  }];
  networking.defaultGateway = "51.161.216.158";
  networking.nameservers = [ 
    "8.8.8.8"
    "8.8.4.4"
  ];


  networking.firewall.enable = true;

  # Hardware configuration
  boot.initrd.availableKernelModules = [ "ata_piix" "uhci_hcd" "virtio_pci" "virtio_scsi" "sd_mod" "sr_mod" ];
  boot.initrd.kernelModules = [ "kvm-amd" ];
  boot.kernelModules = [ ];
  boot.extraModulePackages = [ ];

  fileSystems."/" =
    { device = "/dev/disk/by-uuid/a331eca0-090b-4c81-a6a1-9521dbc66621";
      fsType = "ext4";
    };

  swapDevices = [ ];

  # Bootloader - machine specific
  boot.loader.grub.enable = true;
  boot.loader.grub.device = "/dev/sda";
  boot.loader.grub.useOSProber = true;

  # Set loom-specific secrets file
  sops.defaultSopsFile = ../secrets/loom.yaml;

  # SSH deploy key for auto-update git authentication
  sops.secrets.nixos-auto-deploy-key = {
    owner = "root";
    mode = "0400";
  };

  # Loom server secrets
  sops.secrets.loom-anthropic-api-key = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-openai-api-key = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-github-app-id = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-github-app-private-key = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-github-webhook-secret = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-google-cse-api-key = {
    owner = "loom-server";
    mode = "0400";
  };
  
  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";

  system.stateVersion = "25.11";

  # Auto-update NixOS from git repository
  services.nixos-auto-update = {
    enable = true;
    repository = "git@github.com:ghuntley/loom.git";
    branch = "trunk";
    flakeAttr = "virtualMachine";
    sshKeyFile = config.sops.secrets.nixos-auto-deploy-key.path;
    interval = "*:*";  # every minute
  };

  # Loom Server - API backend
  services.loom-server = {
    enable = true;
    host = "127.0.0.1";
    port = 8080;
    databasePath = "/var/lib/loom-server/loom.db";
    logLevel = "trace";

    anthropic = {
      enable = true;
      apiKeyFile = config.sops.secrets.loom-anthropic-api-key.path;
      model = "claude-sonnet-4-20250514";
    };

    openai = {
      enable = true;
      apiKeyFile = config.sops.secrets.loom-openai-api-key.path;
      model = "gpt-4o";
    };

    githubApp = {
      enable = true;
      appIdFile = config.sops.secrets.loom-github-app-id.path;
      privateKeyFile = config.sops.secrets.loom-github-app-private-key.path;
      webhookSecretFile = config.sops.secrets.loom-github-webhook-secret.path;
    };

    googleCse = {
      enable = true;
      apiKeyFile = config.sops.secrets.loom-google-cse-api-key.path;
      searchEngineId = "017576662512468239146:omuauf_lfve";
    };
  };

  # Loom Web - Web frontend
  services.loom-web = {
    enable = true;
    port = 443;
    serverUrl = "http://127.0.0.1:8080";
    domain = "loom.ghuntley.com";
    enableSSL = true;
    acmeEmail = "ghuntley@ghuntley.com";
  };

  networking.firewall.interfaces."ens18".allowedTCPPorts = [ 80 443 ];

}
