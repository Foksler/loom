# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.loom-smtprelay;
in
{
  options.services.loom-smtprelay = {
    enable = mkEnableOption "SMTP relay service using Grafana smtprelay";

    listenAddress = mkOption {
      type = types.str;
      default = "127.0.0.1:2525";
      description = "Address and port to listen on for incoming SMTP connections.";
    };

    remoteHost = mkOption {
      type = types.str;
      description = "Remote SMTP server to relay emails to (e.g., smtp.gmail.com:587).";
      example = "smtp.gmail.com:587";
    };

    remoteAuthFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Path to file containing remote SMTP authentication in format: username:password
        If null, no authentication is used for the remote server.
      '';
    };

    localAuthFile = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Path to file containing local authentication credentials in bcrypt format.
        If null, no authentication is required for local connections.
      '';
    };

    allowedSenders = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "List of allowed sender email addresses or patterns.";
      example = [ "@example.com$" "^admin@" ];
    };

    allowedRecipients = mkOption {
      type = types.listOf types.str;
      default = [];
      description = "List of allowed recipient email addresses or patterns.";
      example = [ "@example.com$" ];
    };

    logLevel = mkOption {
      type = types.enum [ "debug" "info" "warn" "error" ];
      default = "info";
      description = "Log level for smtprelay.";
    };

    useTLS = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to use STARTTLS when connecting to remote server.";
    };
  };

  config = mkIf cfg.enable {
    users.users.smtprelay = {
      isSystemUser = true;
      group = "smtprelay";
      description = "SMTP Relay service user";
    };

    users.groups.smtprelay = {};

    systemd.services.smtprelay = {
      description = "SMTP Relay Service";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      serviceConfig = {
        Type = "simple";
        User = "smtprelay";
        Group = "smtprelay";
        Restart = "always";
        RestartSec = "5s";

        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
      };

      script = ''
        set -euo pipefail

        ARGS=()
        ARGS+=("-listen" "${cfg.listenAddress}")
        ARGS+=("-remotes" "${cfg.remoteHost}")
        ARGS+=("-log_level" "${cfg.logLevel}")

        ${optionalString cfg.useTLS ''
          ARGS+=("-remote_starttls" "true")
        ''}

        ${optionalString (cfg.remoteAuthFile != null) ''
          REMOTE_CREDS=$(cat ${cfg.remoteAuthFile})
          REMOTE_USER="''${REMOTE_CREDS%%:*}"
          REMOTE_PASS="''${REMOTE_CREDS#*:}"
          ARGS+=("-remote_auth" "plain")
          ARGS+=("-remote_user" "$REMOTE_USER")
          ARGS+=("-remote_pass" "$REMOTE_PASS")
        ''}

        ${optionalString (cfg.localAuthFile != null) ''
          ARGS+=("-local_auth" "${cfg.localAuthFile}")
        ''}

        ${optionalString (cfg.allowedSenders != []) ''
          ARGS+=("-allowed_sender" "${concatStringsSep "," cfg.allowedSenders}")
        ''}

        ${optionalString (cfg.allowedRecipients != []) ''
          ARGS+=("-allowed_recipient" "${concatStringsSep "," cfg.allowedRecipients}")
        ''}

        exec ${pkgs.smtprelay}/bin/smtprelay "''${ARGS[@]}"
      '';
    };
  };
}
