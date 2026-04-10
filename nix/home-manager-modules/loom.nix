{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.loom;
  mysqlCfg = config.services.ephemera.mysql;
  settingsFormat = pkgs.formats.json { };

  mysqlUrl =
    if cfg.mysql != null then
      "mysql://${mysqlCfg.${cfg.mysql}.user}:${mysqlCfg.${cfg.mysql}.password}@localhost:${
        toString mysqlCfg.${cfg.mysql}.port
      }/${mysqlCfg.${cfg.mysql}.database}"
    else
      cfg.mysql_url;
in
{
  options.services.ephemera.loom = {
    enable = lib.mkEnableOption "loom memory service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.loom;
      description = "The loom package to use";
    };

    mysql = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Name of the MySQL instance to use (must be defined in services.ephemera.mysql)";
    };

    mysql_url = lib.mkOption {
      type = lib.types.nullOr lib.types.str;
      default = null;
      description = "Direct MySQL connection URL (used when mysql is null)";
    };

    settings = {
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port for loom service";
      };

      mysql = {
        max_connections = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Maximum number of MySQL connections";
        };
      };
    };

    # Internal option for unified config derivation
    _configJson = lib.mkOption {
      type = lib.types.path;
      internal = true;
    };
  };

  config = {
    services.ephemera.loom._configJson = settingsFormat.generate "loom.json" (
      cfg.settings
      // {
        mysql = cfg.settings.mysql // {
          url = mysqlUrl;
        };
      }
    );

    systemd.user.services.loom = lib.mkIf cfg.enable {
      Unit = {
        Description = "Loom Memory Service";
        After = [
          "network.target"
          "agora.service"
        ]
        ++ lib.optionals (cfg.mysql != null) [ "${cfg.mysql}.service" ];
        Requires = lib.optionals (cfg.mysql != null) [ "${cfg.mysql}.service" ];
      };

      Service = {
        ExecStart = "${cfg.package}/bin/loom --config-dir ${config.services.ephemera._configDir}/loom";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
