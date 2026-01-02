# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.loom-web;
in
{
  options.services.loom-web = {
    enable = mkEnableOption "Loom web application - Browser interface for Loom AI coding assistant";

    package = mkOption {
      type = types.package;
      default = pkgs.loom-web;
      defaultText = literalExpression "pkgs.loom-web";
      description = "The loom-web package to use.";
    };

    domain = mkOption {
      type = types.nullOr types.str;
      default = null;
      example = "loom.example.com";
      description = "Domain name for the web application. If null, served on all domains.";
    };

    port = mkOption {
      type = types.port;
      default = 3000;
      description = "Port for the web server to listen on.";
    };

    openFirewall = mkOption {
      type = types.bool;
      default = false;
      description = "Whether to open the firewall port for loom-web.";
    };

    enableSSL = mkOption {
      type = types.bool;
      default = true;
      description = "Whether to enable HTTPS (requires a domain).";
    };

    acmeEmail = mkOption {
      type = types.nullOr types.str;
      default = null;
      description = "Email address for ACME certificate registration.";
    };

    serverUrl = mkOption {
      type = types.str;
      default = "http://127.0.0.1:8080";
      description = "URL of the loom-server backend for API proxying.";
    };

    extraNginxConfig = mkOption {
      type = types.lines;
      default = "";
      description = "Extra nginx configuration to add to the server block.";
    };
  };

  config = mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.enableSSL -> (cfg.domain != null && cfg.acmeEmail != null);
        message = "services.loom-web.domain and acmeEmail must be set when SSL is enabled.";
      }
    ];

    services.nginx = {
      enable = true;
      recommendedGzipSettings = true;
      recommendedOptimisation = true;
      recommendedProxySettings = true;
      recommendedTlsSettings = mkIf cfg.enableSSL true;

      virtualHosts.${if cfg.domain != null then cfg.domain else "_"} = {
        listen = if cfg.enableSSL && cfg.domain != null then [
          { addr = "0.0.0.0"; port = 80; ssl = false; }
          { addr = "0.0.0.0"; port = 443; ssl = true; }
        ] else [
          { addr = "0.0.0.0"; port = cfg.port; ssl = false; }
        ];

        forceSSL = cfg.enableSSL && cfg.domain != null;
        enableACME = cfg.enableSSL && cfg.domain != null;

        root = "${cfg.package}/share/loom-web";

        locations = {
          "/" = {
            tryFiles = "$uri $uri.html $uri/ /index.html";
          };

          # Proxy API requests to loom-server
          "^~ /api/" = {
            proxyPass = cfg.serverUrl;
            proxyWebsockets = true;
            extraConfig = ''
              proxy_read_timeout 86400;
            '';
          };

          "/proxy/" = {
            proxyPass = cfg.serverUrl;
            proxyWebsockets = true;
            extraConfig = ''
              proxy_read_timeout 86400;
            '';
          };

          "/health" = {
            proxyPass = cfg.serverUrl;
          };

          "/ws" = {
            proxyPass = cfg.serverUrl;
            proxyWebsockets = true;
            extraConfig = ''
              proxy_read_timeout 86400;
            '';
          };

          # CLI binary distribution - exact match takes priority
          "= /bin" = {
            return = "301 /bin/";
          };

          # CLI binary distribution - prefix match with priority modifier
          "^~ /bin/" = {
            proxyPass = cfg.serverUrl;
          };

          # SCM Git HTTP protocol - clone/fetch/push
          "^~ /git/" = {
            proxyPass = cfg.serverUrl;
            extraConfig = ''
              proxy_read_timeout 3600;
              proxy_buffering off;
              client_max_body_size 0;
            '';
          };

          # Static assets with caching
          "~* \\.(js|css|png|jpg|jpeg|gif|ico|svg|woff|woff2|ttf|eot)$" = {
            root = "${cfg.package}/share/loom-web";
            extraConfig = ''
              expires 1y;
              add_header Cache-Control "public, immutable";
            '';
          };
        };

        extraConfig = cfg.extraNginxConfig;
      };
    };

    # ACME configuration for Let's Encrypt
    security.acme = mkIf (cfg.enableSSL && cfg.domain != null) {
      acceptTerms = true;
      defaults.email = cfg.acmeEmail;
    };

    networking.firewall = mkIf cfg.openFirewall {
      allowedTCPPorts = [ cfg.port ] ++ (optionals cfg.enableSSL [ 443 80 ]);
    };
  };
}
