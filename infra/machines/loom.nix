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
    ../nixos-modules/k3s.nix
    ../nixos-modules/maxmind-geoip-update.nix
    ../nixos-modules/smtprelay.nix
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
  networking.defaultGateway = {
    address = "51.161.216.158";
    interface = "eth0";
  };
 
  networking.nameservers = [ 
    "8.8.8.8"
    "8.8.4.4"
  ];


  networking.firewall.enable = false;

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

  sops.secrets.loom-github-app-client-id = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-github-app-client-secret = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-google-cse-api-key = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.loom-google-cse-search-engine-id = {
    owner = "loom-server";
    mode = "0400";
  };

  sops.secrets.maxmind-account-id = {
    owner = "root";
    mode = "0400";
  };

  sops.secrets.maxmind-license-key = {
    owner = "root";
    mode = "0400";
  };

  sops.secrets.smtp-relay-auth = {
    owner = "smtprelay";
    group = "smtprelay";
    mode = "0400";
  };

  nixpkgs.hostPlatform = lib.mkDefault "x86_64-linux";

  system.stateVersion = "25.11";

  # K3s Kubernetes cluster
  services.loom-k3s = {
    enable = true;
    role = "server";
    clusterInit = true;
    disableTraefik = true;  # We use nginx via loom-web
  };

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
    binDir = pkgs.loom-cli-binaries;
    baseUrl = "https://loom.ghuntley.com";

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

    githubOAuth = {
      enable = true;
      clientIdFile = config.sops.secrets.loom-github-app-client-id.path;
      clientSecretFile = config.sops.secrets.loom-github-app-client-secret.path;
      redirectUri = "https://loom.ghuntley.com/auth/github/callback";
    };

    googleCse = {
      enable = true;
      apiKeyFile = config.sops.secrets.loom-google-cse-api-key.path;
      searchEngineIdFile = config.sops.secrets.loom-google-cse-search-engine-id.path;
    };

    weaver = {
      enable = true;
      namespace = "loom-weavers";
    };

    geoip = {
      enable = true;
    };

    smtp = {
      enable = true;
      host = "127.0.0.1";
      port = 2525;
      fromAddress = "noreply@loom.ghuntley.com";
      fromName = "Loom";
      useTLS = true;
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

  # MaxMind GeoIP database updates
  services.loom-geoipupdate = {
    enable = true;
    accountIdFile = config.sops.secrets.maxmind-account-id.path;
    licenseKeyFile = config.sops.secrets.maxmind-license-key.path;
  };

  # SMTP Relay - forwards emails to external SMTP server (smtp2go)
  services.loom-smtprelay = {
    enable = true;
    listenAddress = "127.0.0.1:2525";
    remoteHost = "mail-au.smtp2go.com:2525";
    remoteAuthFile = config.sops.secrets.smtp-relay-auth.path;
    useTLS = true;
  };
}
