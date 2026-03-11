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

  # Build the MySQL URL from the referenced mysql instance
  mysqlUrl = "mysql://${mysqlCfg.${cfg.mysql}.user}:${mysqlCfg.${cfg.mysql}.password}@localhost:${
    toString mysqlCfg.${cfg.mysql}.port
  }/${mysqlCfg.${cfg.mysql}.database}";
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
      type = lib.types.str;
      description = "Name of the MySQL instance to use (must be defined in services.ephemera.mysql)";
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
          "${cfg.mysql}.service"
          "agora.service"
        ];
        Requires = [ "${cfg.mysql}.service" ];
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
