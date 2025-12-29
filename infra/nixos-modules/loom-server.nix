# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.loom-server;
in
{
  options.services.loom-server = {
    enable = mkEnableOption "Loom server - HTTP server for Loom AI coding assistant";

    package = mkOption {
      type = types.package;
      default = pkgs.loom-server;
      defaultText = literalExpression "pkgs.loom-server";
      description = "The loom-server package to use.";
    };

    host = mkOption {
      type = types.str;
      default = "127.0.0.1";
      description = "Address to bind the server to.";
    };

    port = mkOption {
      type = types.port;
      default = 8080;
      description = "Port to listen on.";
    };

    databasePath = mkOption {
      type = types.path;
      default = "/var/lib/loom-server/loom.db";
      description = "Path to the SQLite database file.";
    };

    logLevel = mkOption {
      type = types.enum [ "trace" "debug" "info" "warn" "error" ];
      default = "info";
      description = "Log level for the server.";
    };

    binDir = mkOption {
      type = types.nullOr types.path;
      default = null;
      description = ''
        Directory containing CLI binaries for distribution.
        Binaries are served at /bin/{platform} for self-update functionality.
        Expected structure: bin/linux-x86_64, bin/macos-aarch64, etc.
      '';
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to open the firewall port for loom-server.";
    };

    # LLM Provider Configuration
    anthropic = {
      enable = mkEnableOption "Anthropic Claude provider";

      apiKeyFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing Anthropic API key.";
      };

      model = mkOption {
        type = types.str;
        default = "claude-sonnet-4-20250514";
        description = "Anthropic model to use.";
      };
    };

    openai = {
      enable = mkEnableOption "OpenAI provider";

      apiKeyFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing OpenAI API key.";
      };

      model = mkOption {
        type = types.str;
        default = "gpt-4o";
        description = "OpenAI model to use.";
      };

      organization = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "OpenAI organization ID.";
      };
    };

    vertex = {
      enable = mkEnableOption "Google Vertex AI provider";

      projectId = mkOption {
        type = types.nullOr types.str;
        default = null;
        description = "Google Cloud project ID.";
      };

      location = mkOption {
        type = types.str;
        default = "us-central1";
        description = "Google Cloud region.";
      };

      credentialsFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to Google Cloud service account credentials JSON file.";
      };
    };

    # GitHub App Configuration
    githubApp = {
      enable = mkEnableOption "GitHub App integration";

      appIdFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing GitHub App ID.";
      };

      privateKeyFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing GitHub App private key.";
      };

      webhookSecretFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing GitHub webhook secret.";
      };
    };

    # Google Custom Search Engine Configuration
    googleCse = {
      enable = mkEnableOption "Google Custom Search Engine";

      apiKeyFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing Google API key.";
      };

      searchEngineIdFile = mkOption {
        type = types.nullOr types.path;
        default = null;
        description = "Path to file containing Google Custom Search Engine ID.";
      };
    };

    # Weaver Provisioner Configuration
    weaver = {
      enable = mkEnableOption "Weaver provisioner for Kubernetes-based code execution environments";

      namespace = mkOption {
        type = types.str;
        default = "loom-weavers";
        description = "Kubernetes namespace for weavers.";
      };

      kubeconfigPath = mkOption {
        type = types.path;
        default = "/etc/rancher/k3s/k3s.yaml";
        description = ''
          Path to kubeconfig file.
          Note: This file must exist and be readable by the loom-server user.
          When using k3s, ensure the loom-server user is in the loom-k3s group.
        '';
      };

      cleanupIntervalSecs = mkOption {
        type = types.int;
        default = 1800;
        description = "Cleanup interval in seconds for expired weavers.";
      };

      defaultTtlHours = mkOption {
        type = types.int;
        default = 4;
        description = "Default TTL in hours for weavers.";
      };

      maxTtlHours = mkOption {
        type = types.int;
        default = 48;
        description = "Maximum TTL in hours for weavers.";
      };

      maxConcurrent = mkOption {
        type = types.int;
        default = 64;
        description = "Maximum number of concurrent weavers.";
      };

      readyTimeoutSecs = mkOption {
        type = types.int;
        default = 60;
        description = "Timeout in seconds waiting for weaver to become ready.";
      };

      webhooks = mkOption {
        type = types.str;
        default = "[]";
        description = "JSON string of webhook configurations.";
      };
    };

    extraEnvironment = mkOption {
      type = types.attrsOf types.str;
      default = { };
      description = "Extra environment variables to pass to the server.";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.anthropic.enable -> cfg.anthropic.apiKeyFile != null;
        message = "services.loom-server.anthropic.apiKeyFile must be set when Anthropic is enabled.";
      }
      {
        assertion = cfg.openai.enable -> cfg.openai.apiKeyFile != null;
        message = "services.loom-server.openai.apiKeyFile must be set when OpenAI is enabled.";
      }
      {
        assertion = cfg.vertex.enable -> (cfg.vertex.projectId != null && cfg.vertex.credentialsFile != null);
        message = "services.loom-server.vertex.projectId and credentialsFile must be set when Vertex AI is enabled.";
      }
      {
        assertion = cfg.githubApp.enable -> (cfg.githubApp.appIdFile != null && cfg.githubApp.privateKeyFile != null);
        message = "services.loom-server.githubApp.appIdFile and privateKeyFile must be set when GitHub App is enabled.";
      }
      {
        assertion = cfg.googleCse.enable -> (cfg.googleCse.apiKeyFile != null && cfg.googleCse.searchEngineIdFile != null);
        message = "services.loom-server.googleCse.apiKeyFile and searchEngineIdFile must be set when Google CSE is enabled.";
      }

    ];

    users.users.loom-server = {
      isSystemUser = true;
      group = "loom-server";
      home = "/var/lib/loom-server";
      createHome = true;
      description = "Loom server service user";
      extraGroups = mkIf cfg.weaver.enable [ "loom-k3s" ];
    };

    users.groups.loom-server = { };

    systemd.services.loom-server = {
      description = "Loom Server - HTTP server for Loom AI coding assistant";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      wantedBy = [ "multi-user.target" ];

      environment = mkMerge [
        {
          LOOM_SERVER_HOST = cfg.host;
          LOOM_SERVER_PORT = toString cfg.port;
          LOOM_SERVER_DATABASE_URL = "sqlite:${cfg.databasePath}";
          RUST_LOG = cfg.logLevel;
        }
        (mkIf cfg.anthropic.enable {
          LOOM_SERVER_ANTHROPIC_MODEL = cfg.anthropic.model;
        })
        (mkIf cfg.openai.enable {
          LOOM_SERVER_OPENAI_MODEL = cfg.openai.model;
        })
        (mkIf (cfg.openai.enable && cfg.openai.organization != null) {
          LOOM_SERVER_OPENAI_ORG = cfg.openai.organization;
        })
        (mkIf cfg.vertex.enable {
          LOOM_SERVER_VERTEX_PROJECT_ID = cfg.vertex.projectId;
          LOOM_SERVER_VERTEX_LOCATION = cfg.vertex.location;
        })

        (mkIf (cfg.binDir != null) {
          LOOM_SERVER_BIN_DIR = toString cfg.binDir;
        })
        (mkIf cfg.weaver.enable {
          LOOM_SERVER_WEAVER_ENABLED = "true";
          LOOM_SERVER_WEAVER_K8S_NAMESPACE = cfg.weaver.namespace;
          LOOM_SERVER_WEAVER_CLEANUP_INTERVAL_SECS = toString cfg.weaver.cleanupIntervalSecs;
          LOOM_SERVER_WEAVER_DEFAULT_TTL_HOURS = toString cfg.weaver.defaultTtlHours;
          LOOM_SERVER_WEAVER_MAX_TTL_HOURS = toString cfg.weaver.maxTtlHours;
          LOOM_SERVER_WEAVER_MAX_CONCURRENT = toString cfg.weaver.maxConcurrent;
          LOOM_SERVER_WEAVER_READY_TIMEOUT_SECS = toString cfg.weaver.readyTimeoutSecs;
          LOOM_SERVER_WEAVER_WEBHOOKS = cfg.weaver.webhooks;
          KUBECONFIG = toString cfg.weaver.kubeconfigPath;
        })
        cfg.extraEnvironment
      ];

      script = let
        loadSecret = file: envVar: optionalString (file != null) ''
          export ${envVar}="$(cat ${file})"
        '';
      in ''
        ${loadSecret cfg.anthropic.apiKeyFile "LOOM_SERVER_ANTHROPIC_API_KEY"}
        ${loadSecret cfg.openai.apiKeyFile "LOOM_SERVER_OPENAI_API_KEY"}
        ${loadSecret cfg.vertex.credentialsFile "GOOGLE_APPLICATION_CREDENTIALS"}
        ${loadSecret cfg.githubApp.appIdFile "LOOM_GITHUB_APP_ID"}
        ${loadSecret cfg.githubApp.privateKeyFile "LOOM_GITHUB_APP_PRIVATE_KEY_FILE"}
        ${loadSecret cfg.githubApp.webhookSecretFile "LOOM_GITHUB_WEBHOOK_SECRET"}
        ${loadSecret cfg.googleCse.apiKeyFile "LOOM_SERVER_GOOGLE_CSE_API_KEY"}
        ${loadSecret cfg.googleCse.searchEngineIdFile "LOOM_SERVER_GOOGLE_CSE_SEARCH_ENGINE_ID"}

        exec ${cfg.package}/bin/loom-server
      '';

      serviceConfig = {
        Type = "simple";
        User = "loom-server";
        Group = "loom-server";
        WorkingDirectory = "/var/lib/loom-server";
        StateDirectory = "loom-server";
        RuntimeDirectory = "loom-server";

        # Security hardening
        NoNewPrivileges = true;
        ProtectSystem = "strict";
        ProtectHome = true;
        PrivateTmp = true;
        PrivateDevices = true;
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectControlGroups = true;
        RestrictAddressFamilies = [ "AF_INET" "AF_INET6" "AF_UNIX" ];
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        MemoryDenyWriteExecute = true;
        LockPersonality = true;
        SystemCallArchitectures = "native";
        SystemCallFilter = [ "@system-service" "~@privileged" "~@resources" ];

        # Restart policy
        Restart = "on-failure";
        RestartSec = "5s";

        StandardOutput = "journal";
        StandardError = "journal";
      };
    };
  };
}
