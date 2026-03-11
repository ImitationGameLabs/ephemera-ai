{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.agora;
  settingsFormat = pkgs.formats.json { };
in
{
  options.services.ephemera.agora = {
    enable = lib.mkEnableOption "agora event hub service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.agora;
      description = "The agora package to use";
    };

    settings = {
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port for agora service";
      };

      database_path = lib.mkOption {
        type = lib.types.str;
        description = ''
          Path to SQLite database file.

          For user services, prefer XDG data directory:
            `${"\${config.xdg.dataHome}"}/ephemera/agora.db`
          which resolves to `~/.local/share/ephemera/agora.db` by default.
        '';
      };

      heartbeat_check_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Interval between heartbeat timeout checks (ms)";
      };

      timeout_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Milliseconds before marking herald as Disconnected";
      };

      retry = {
        base_interval_ms = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Initial retry interval (ms)";
        };

        multiplier = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Multiplier for each retry";
        };

        max_interval_ms = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Max retry interval (ms)";
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
    services.ephemera.agora._configJson = settingsFormat.generate "agora.json" cfg.settings;

    systemd.user.services.agora = lib.mkIf cfg.enable {
      Unit = {
        Description = "Agora Event Hub";
        After = [ "network.target" ];
      };

      Service = {
        # Ensure parent directory exists before starting the service
        ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p ${dirOf cfg.settings.database_path}";
        ExecStart = "${cfg.package}/bin/agora --config-dir ${config.services.ephemera._configDir}/agora";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
