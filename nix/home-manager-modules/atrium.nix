{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.atrium;
  mysqlCfg = config.services.ephemera.mysql;
  settingsFormat = pkgs.formats.json { };

  # Build the MySQL URL from the referenced mysql instance
  mysqlUrl = "mysql://${mysqlCfg.${cfg.mysql}.user}:${mysqlCfg.${cfg.mysql}.password}@localhost:${
    toString mysqlCfg.${cfg.mysql}.port
  }/${mysqlCfg.${cfg.mysql}.database}";
in
{
  options.services.ephemera.atrium = {
    enable = lib.mkEnableOption "atrium chat service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.atrium;
      description = "The atrium package to use";
    };

    cliPackage = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.atrium-cli;
      description = "The atrium-cli package to use";
    };

    heraldPackage = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.atrium-herald;
      description = "The atrium-herald package to use";
    };

    mysql = lib.mkOption {
      type = lib.types.str;
      description = "Name of the MySQL instance to use (must be defined in services.ephemera.mysql)";
    };

    settings = {
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port for atrium service";
      };
    };

    heraldSettings = {
      atrium_url = lib.mkOption {
        type = lib.types.str;
        description = "Atrium service URL";
      };

      agora_url = lib.mkOption {
        type = lib.types.str;
        description = "Agora service URL";
      };

      username = lib.mkOption {
        type = lib.types.str;
        description = "Atrium herald login username";
      };

      password = lib.mkOption {
        type = lib.types.str;
        description = "Atrium herald login password";
      };

      poll_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Message poll interval (ms)";
      };

      heartbeat_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Agora heartbeat interval (ms)";
      };

      atrium_heartbeat_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Atrium heartbeat interval for user online status (ms)";
      };

      bio = lib.mkOption {
        type = lib.types.str;
        description = "Bio for user registration (use empty string if not needed)";
      };
    };

    # Internal options for unified config derivation
    _configJson = lib.mkOption {
      type = lib.types.path;
      internal = true;
    };

    _heraldConfigJson = lib.mkOption {
      type = lib.types.path;
      internal = true;
    };
  };

  config = {
    services.ephemera.atrium._configJson = settingsFormat.generate "atrium.json" (
      cfg.settings // { mysql_url = mysqlUrl; }
    );

    services.ephemera.atrium._heraldConfigJson = settingsFormat.generate "config.json" cfg.heraldSettings;

    # Auto-include atrium-cli
    home.packages = lib.mkIf cfg.enable [ cfg.cliPackage ];

    # Atrium service
    systemd.user.services.atrium = lib.mkIf cfg.enable {
      Unit = {
        Description = "Atrium Chat Service";
        After = [
          "network.target"
          "${cfg.mysql}.service"
        ];
        Requires = [ "${cfg.mysql}.service" ];
      };

      Service = {
        ExecStart = "${cfg.package}/bin/atrium --config-dir ${config.services.ephemera._configDir}/atrium";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };

    # Atrium Herald service
    systemd.user.services.atrium-herald = lib.mkIf cfg.enable {
      Unit = {
        Description = "Atrium Herald";
        After = [
          "atrium.service"
          "agora.service"
        ];
        Requires = [ "atrium.service" ];
      };

      Service = {
        ExecStart = "${cfg.heraldPackage}/bin/atrium-herald --config-dir ${config.services.ephemera._configDir}/atrium-herald";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
