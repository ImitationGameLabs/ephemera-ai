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

  # Generate JSON config file for loom
  loomConfig = pkgs.writeText "loom.json" (
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

  # Create config directory with the JSON file
  configDir = pkgs.runCommand "loom-config" { } ''
    mkdir -p $out
    ln -s ${loomConfig} $out/loom.json
  '';
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
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.loom = {
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
        ExecStart = "${cfg.package}/bin/loom --config-dir ${configDir}";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
