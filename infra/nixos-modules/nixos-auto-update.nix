# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.nixos-auto-update;
in
{
  options.services.nixos-auto-update = {
    enable = mkEnableOption "NixOS auto-update service";

    repository = mkOption {
      type = types.str;
      default = "https://github.com/ghuntley/ghuntley.git";
      description = "Git repository URL to clone/update";
    };

    branch = mkOption {
      type = types.str;
      default = "main";
      description = "Git branch to track";
    };

    localPath = mkOption {
      type = types.path;
      default = "/var/lib/depot";
      description = "Local path where the repository will be cloned";
    };

    flakeAttr = mkOption {
      type = types.str;
      default = "virtualMachine";
      description = "Flake attribute to activate (nixosConfigurations.<attr>)";
    };

    interval = mkOption {
      type = types.str;
      default = "*:0/5";
      description = "Systemd calendar expression for update interval (default: every 5 minutes)";
    };

    sshKeyFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = "Path to SSH private key for git authentication (optional)";
    };
  };

  config = mkIf cfg.enable {
    systemd.services.nixos-auto-update = {
      description = "Auto-update repository and activate NixOS flake";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];

      path = with pkgs; [ git nix nixos-rebuild openssh ];

      environment = mkMerge [
        { HOME = "/root"; }
        (mkIf (cfg.sshKeyFile != null) {
          GIT_SSH_COMMAND = "ssh -i ${cfg.sshKeyFile} -o StrictHostKeyChecking=accept-new";
        })
      ];

      script = ''
        set -euo pipefail

        REPO_PATH="${cfg.localPath}"
        REPO_URL="${cfg.repository}"
        BRANCH="${cfg.branch}"
        FLAKE_ATTR="${cfg.flakeAttr}"

        echo "[$(date -Iseconds)] Starting nixos-auto-update..."

        # Clone if not exists, otherwise fetch and reset
        if [ ! -d "$REPO_PATH/.git" ]; then
          echo "Cloning repository..."
          git clone --branch "$BRANCH" --single-branch "$REPO_URL" "$REPO_PATH"
        else
          echo "Updating repository..."
          cd "$REPO_PATH"
          git fetch origin "$BRANCH"
          
          LOCAL_REV=$(git rev-parse HEAD)
          REMOTE_REV=$(git rev-parse "origin/$BRANCH")
          
          if [ "$LOCAL_REV" = "$REMOTE_REV" ]; then
            echo "Already up to date at $LOCAL_REV"
            exit 0
          fi
          
          echo "Updating from $LOCAL_REV to $REMOTE_REV"
          git reset --hard "origin/$BRANCH"
        fi

        cd "$REPO_PATH"
        CURRENT_REV=$(git rev-parse HEAD)
        echo "At revision: $CURRENT_REV"

        echo "Activating flake..."
        nixos-rebuild switch --flake ".#$FLAKE_ATTR"

        echo "[$(date -Iseconds)] Auto-update complete"
      '';

      serviceConfig = {
        Type = "oneshot";
        User = "root";
        Group = "root";
        StandardOutput = "journal";
        StandardError = "journal";
      };
    };

    systemd.timers.nixos-auto-update = {
      description = "Timer for nixos-auto-update";
      wantedBy = [ "timers.target" ];

      timerConfig = {
        OnCalendar = cfg.interval;
        Persistent = true;
        RandomizedDelaySec = "30s";
      };
    };

    environment.systemPackages = with pkgs; [ git ];
  };
}
