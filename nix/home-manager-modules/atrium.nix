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

  # Build the MySQL URL
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

    port = lib.mkOption {
      type = lib.types.port;
      description = "Port for atrium service";
    };

    mysql = lib.mkOption {
      type = lib.types.str;
      description = "Name of the MySQL instance to use (must be defined in services.ephemera.mysql)";
    };

    agoraUrl = lib.mkOption {
      type = lib.types.str;
      description = "Agora service URL";
    };

    heraldAuth = {
      username = lib.mkOption {
        type = lib.types.str;
        description = "Atrium herald login username";
      };

      password = lib.mkOption {
        type = lib.types.str;
        description = "Atrium herald login password";
      };

      bio = lib.mkOption {
        type = lib.types.nullOr lib.types.str;
        description = "Bio for user registration (optional)";
      };
    };

    heraldPollIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Message poll interval (ms)";
    };

    heraldHeartbeatIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Agora heartbeat interval (ms)";
    };

    heraldAtriumHeartbeatIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Atrium heartbeat interval for user online status (ms)";
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
    services.ephemera.atrium._configJson = pkgs.writeText "atrium.json" (
      builtins.toJSON {
        port = cfg.port;
        mysql_url = mysqlUrl;
      }
    );

    services.ephemera.atrium._heraldConfigJson = pkgs.writeText "config.json" (
      builtins.toJSON (
        {
          atrium_url = "http://localhost:${toString cfg.port}";
          agora_url = cfg.agoraUrl;
          username = cfg.heraldAuth.username;
          password = cfg.heraldAuth.password;
          poll_interval_ms = cfg.heraldPollIntervalMs;
          heartbeat_interval_ms = cfg.heraldHeartbeatIntervalMs;
          atrium_heartbeat_interval_ms = cfg.heraldAtriumHeartbeatIntervalMs;
        }
        // lib.optionalAttrs (cfg.heraldAuth.bio != null) {
          bio = cfg.heraldAuth.bio;
        }
      )
    );

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
