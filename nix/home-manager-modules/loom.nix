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

  # Build the MySQL URL
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

    port = lib.mkOption {
      type = lib.types.port;
      description = "Port for loom service";
    };

    mysql = lib.mkOption {
      type = lib.types.str;
      description = "Name of the MySQL instance to use (must be defined in services.ephemera.mysql)";
    };

    mysqlMaxConnections = lib.mkOption {
      type = lib.types.nullOr lib.types.ints.positive;
      description = "Maximum number of MySQL connections (optional)";
    };

    agoraUrl = lib.mkOption {
      type = lib.types.str;
      description = "Agora service URL";
    };

    # Internal option for unified config derivation
    _configJson = lib.mkOption {
      type = lib.types.path;
      internal = true;
    };
  };

  config = {
    services.ephemera.loom._configJson = pkgs.writeText "loom.json" (
      builtins.toJSON {
        port = cfg.port;
        mysql = {
          url = mysqlUrl;
        }
        // lib.optionalAttrs (cfg.mysqlMaxConnections != null) {
          max_connections = cfg.mysqlMaxConnections;
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
