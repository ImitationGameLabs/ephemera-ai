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

  mysqlUrl =
    if cfg.mysql != null then
      "mysql://${mysqlCfg.${cfg.mysql}.user}:${mysqlCfg.${cfg.mysql}.password}@localhost:${
        toString mysqlCfg.${cfg.mysql}.port
      }/${mysqlCfg.${cfg.mysql}.database}"
    else
      cfg.mysql_url;

  serverUrl = "http://localhost:${toString cfg.settings.port}";

  heraldConfig = {
    atrium_url = serverUrl;
    agora_url = cfg.herald.agora_url;
    username = cfg.auth.username;
    password = cfg.auth.password;
    bio = cfg.auth.bio;
    poll_interval_ms = cfg.herald.poll_interval_ms;
    heartbeat_interval_ms = cfg.herald.heartbeat_interval_ms;
    atrium_heartbeat_interval_ms = cfg.herald.atrium_heartbeat_interval_ms;
  };

  cliDefaultConfig = settingsFormat.generate "config.json" {
    "server-url" = serverUrl;
    auth = {
      username = cfg.auth.username;
      password = cfg.auth.password;
    } // lib.optionalAttrs (cfg.auth.bio != "") { bio = cfg.auth.bio; };
  };
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

    log_level = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = ''
        Log filter directive passed as RUST_LOG to all atrium service processes
        (atrium and atrium-herald). Accepts a plain level ("info", "debug", "warn")
        or a comma-separated EnvFilter directive for fine-grained per-crate control
        (e.g. "debug,hyper=warn,sqlx=warn").
      '';
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
        description = "Port for atrium service";
      };
    };

    auth = {
      username = lib.mkOption {
        type = lib.types.str;
        description = "Atrium username (used by atrium-herald and as atrium-cli default)";
      };

      password = lib.mkOption {
        type = lib.types.str;
        description = "Atrium password (used by atrium-herald and as atrium-cli default)";
      };

      bio = lib.mkOption {
        type = lib.types.str;
        default = "";
        description = "Bio for user registration";
      };
    };

    herald = {
      agora_url = lib.mkOption {
        type = lib.types.str;
        description = "Agora service URL";
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

    services.ephemera.atrium._heraldConfigJson = settingsFormat.generate "config.json" heraldConfig;

    # Auto-include atrium-cli
    home.packages = lib.mkIf cfg.enable [ cfg.cliPackage ];

    # Write default atrium-cli config if not already present
    home.activation.atriumCliConfig = lib.mkIf cfg.enable (
      lib.dag.entryAfter [ "writeBoundary" ] ''
        if [ ! -f "$HOME/.config/atrium-cli/config.json" ]; then
          mkdir -p "$HOME/.config/atrium-cli"
          cp ${cliDefaultConfig} "$HOME/.config/atrium-cli/config.json"
        fi
      ''
    );

    # Atrium service
    systemd.user.services.atrium = lib.mkIf cfg.enable {
      Unit = {
        Description = "Atrium Chat Service";
        After = [
          "network.target"
        ]
        ++ lib.optionals (cfg.mysql != null) [ "${cfg.mysql}.service" ];
        Requires = lib.optionals (cfg.mysql != null) [ "${cfg.mysql}.service" ];
      };

      Service = {
        Environment = [ "RUST_LOG=${cfg.log_level}" ];
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
        Environment = [ "RUST_LOG=${cfg.log_level}" ];
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
