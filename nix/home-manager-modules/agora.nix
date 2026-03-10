{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.agora;

  # Generate JSON config file for agora
  agoraConfig = pkgs.writeText "agora.json" (
    builtins.toJSON {
      port = cfg.port;
      database_path = cfg.databasePath;
      heartbeat_check_interval_ms = cfg.heartbeatCheckIntervalMs;
      timeout_ms = cfg.timeoutMs;
      retry = {
        base_interval_ms = cfg.retry.baseIntervalMs;
        multiplier = cfg.retry.multiplier;
        max_interval_ms = cfg.retry.maxIntervalMs;
      };
    }
  );

  # Create config directory with the JSON file
  configDir = pkgs.runCommand "agora-config" { } ''
    mkdir -p $out
    ln -s ${agoraConfig} $out/agora.json
  '';
in
{
  options.services.ephemera.agora = {
    enable = lib.mkEnableOption "agora event hub service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.agora;
      description = "The agora package to use";
    };

    port = lib.mkOption {
      type = lib.types.port;
      description = "Port for agora service";
    };

    databasePath = lib.mkOption {
      type = lib.types.str;
      description = ''
        Path to SQLite database file.

        For user services, prefer XDG data directory:
          `${"\${config.xdg.dataHome}"}/ephemera/agora.db`
        which resolves to `~/.local/share/ephemera/agora.db` by default.
      '';
    };

    heartbeatCheckIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Interval between heartbeat timeout checks (ms)";
    };

    timeoutMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Milliseconds before marking herald as Disconnected";
    };

    retry = {
      baseIntervalMs = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Initial retry interval (ms)";
      };

      multiplier = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Multiplier for each retry";
      };

      maxIntervalMs = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Max retry interval (ms)";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.agora = {
      Unit = {
        Description = "Agora Event Hub";
        After = [ "network.target" ];
      };

      Service = {
        # Ensure parent directory exists before starting the service
        ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p ${builtins.dirOf cfg.databasePath}";
        ExecStart = "${cfg.package}/bin/agora --config-dir ${configDir}";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
