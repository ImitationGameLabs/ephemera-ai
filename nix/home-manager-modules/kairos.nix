{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.kairos;

  # Generate JSON config file for kairos
  kairosConfig = pkgs.writeText "kairos.json" (
    builtins.toJSON {
      port = cfg.port;
      database_path = cfg.databasePath;
      tick_interval_ms = cfg.tickIntervalMs;
    }
  );

  # Create config directory with the JSON file
  configDir = pkgs.runCommand "kairos-config" { } ''
    mkdir -p $out
    ln -s ${kairosConfig} $out/kairos.json
  '';

  # Generate JSON config file for kairos-herald
  heraldConfig = pkgs.writeText "config.json" (
    builtins.toJSON {
      kairos_url = "http://localhost:${toString cfg.port}";
      agora_url = cfg.agoraUrl;
      poll_interval_ms = cfg.heraldPollIntervalMs;
      heartbeat_interval_sec = cfg.heraldHeartbeatIntervalSec;
    }
  );

  # Create config directory for kairos-herald
  heraldConfigDir = pkgs.runCommand "kairos-herald-config" { } ''
    mkdir -p $out
    ln -s ${heraldConfig} $out/config.json
  '';
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
  };

  config = lib.mkIf cfg.enable {
    # Auto-include kairos-cli
    home.packages = [ cfg.cliPackage ];

    # Kairos service
    systemd.user.services.kairos = {
      Unit = {
        Description = "Kairos Time Service";
        After = [ "network.target" ];
      };

      Service = {
        # Ensure parent directory exists before starting the service
        ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p ${builtins.dirOf cfg.databasePath}";
        ExecStart = "${cfg.package}/bin/kairos --config-dir ${configDir}";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };

    # Kairos Herald service
    systemd.user.services.kairos-herald = {
      Unit = {
        Description = "Kairos Herald";
        After = [
          "kairos.service"
          "agora.service"
        ];
        Requires = [ "kairos.service" ];
      };

      Service = {
        ExecStart = "${cfg.heraldPackage}/bin/kairos-herald --config-dir ${heraldConfigDir}";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
