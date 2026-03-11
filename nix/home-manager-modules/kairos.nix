{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.kairos;
in
{
  options.services.ephemera.kairos = {
    enable = lib.mkEnableOption "kairos time service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.kairos;
      description = "The kairos package to use";
    };

    cliPackage = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.kairos-cli;
      description = "The kairos-cli package to use";
    };

    heraldPackage = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.kairos-herald;
      description = "The kairos-herald package to use";
    };

    port = lib.mkOption {
      type = lib.types.port;
      description = "Port for kairos service";
    };

    databasePath = lib.mkOption {
      type = lib.types.str;
      description = ''
        Path to SQLite database file.

        For user services, prefer XDG data directory:
          `${"\${config.xdg.dataHome}"}/ephemera/kairos.db`
        which resolves to `~/.local/share/ephemera/kairos.db` by default.
      '';
    };

    tickIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Interval for checking scheduled events (ms)";
    };

    agoraUrl = lib.mkOption {
      type = lib.types.str;
      description = "Agora service URL";
    };

    heraldPollIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Poll interval for triggered schedules (ms)";
    };

    heraldHeartbeatIntervalSec = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Heartbeat interval in seconds";
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
    services.ephemera.kairos._configJson = pkgs.writeText "kairos.json" (
      builtins.toJSON {
        port = cfg.port;
        database_path = cfg.databasePath;
        tick_interval_ms = cfg.tickIntervalMs;
      }
    );

    services.ephemera.kairos._heraldConfigJson = pkgs.writeText "config.json" (
      builtins.toJSON {
        kairos_url = "http://localhost:${toString cfg.port}";
        agora_url = cfg.agoraUrl;
        poll_interval_ms = cfg.heraldPollIntervalMs;
        heartbeat_interval_sec = cfg.heraldHeartbeatIntervalSec;
      }
    );

    # Auto-include kairos-cli
    home.packages = lib.mkIf cfg.enable [ cfg.cliPackage ];

    # Kairos service
    systemd.user.services.kairos = lib.mkIf cfg.enable {
      Unit = {
        Description = "Kairos Time Service";
        After = [ "network.target" ];
      };

      Service = {
        # Ensure parent directory exists before starting the service
        ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p ${builtins.dirOf cfg.databasePath}";
        ExecStart = "${cfg.package}/bin/kairos --config-dir ${config.services.ephemera._configDir}/kairos";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };

    # Kairos Herald service
    systemd.user.services.kairos-herald = lib.mkIf cfg.enable {
      Unit = {
        Description = "Kairos Herald";
        After = [
          "kairos.service"
          "agora.service"
        ];
        Requires = [ "kairos.service" ];
      };

      Service = {
        ExecStart = "${cfg.heraldPackage}/bin/kairos-herald --config-dir ${config.services.ephemera._configDir}/kairos-herald";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
