# Copyright (c) 2025 Geoffrey Huntley <ghuntley@ghuntley.com>. All rights reserved.
# SPDX-License-Identifier: Proprietary

{ config, lib, pkgs, ... }:

with lib;

let
  cfg = config.services.loom-geoipupdate;
in
{
  options.services.loom-geoipupdate = {
    enable = mkEnableOption "MaxMind GeoIP database updates";

    stateDir = mkOption {
      type = types.path;
      default = "/var/lib/GeoIP";
      description = "Directory where GeoIP databases are stored.";
    };

    accountIdFile = mkOption {
      type = types.path;
      description = "Path to file containing the MaxMind Account ID.";
      example = "config.sops.secrets.maxmind-account-id.path";
    };

    licenseKeyFile = mkOption {
      type = types.path;
      description = "Path to file containing the MaxMind license key.";
      example = "config.sops.secrets.maxmind-license-key.path";
    };

    editionIds = mkOption {
      type = types.listOf types.str;
      default = [
        "GeoLite2-ASN"
        "GeoLite2-City"
        "GeoLite2-Country"
      ];
      description = "List of GeoIP database edition IDs to download.";
    };
  };

  config = mkIf cfg.enable {
    services.geoipupdate = {
      enable = true;
      settings = {
        AccountID = cfg.accountIdFile;
        LicenseKey = cfg.licenseKeyFile;
        EditionIDs = cfg.editionIds;
        DatabaseDirectory = cfg.stateDir;
      };
    };
  };
}
