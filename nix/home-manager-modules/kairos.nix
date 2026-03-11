{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.kairos;
  settingsFormat = pkgs.formats.json { };
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

    settings = {
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port for kairos service";
      };

      database_path = lib.mkOption {
        type = lib.types.str;
        description = ''
          Path to SQLite database file.

          For user services, prefer XDG data directory:
            `${"\${config.xdg.dataHome}"}/ephemera/kairos.db`
          which resolves to `~/.local/share/ephemera/kairos.db` by default.
        '';
      };

      tick_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Interval for checking scheduled events (ms)";
      };
    };

    heraldSettings = {
      kairos_url = lib.mkOption {
        type = lib.types.str;
        description = "Kairos service URL";
      };

      agora_url = lib.mkOption {
        type = lib.types.str;
        description = "Agora service URL";
      };

      poll_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Poll interval for triggered schedules (ms)";
      };

      heartbeat_interval_sec = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Heartbeat interval in seconds";
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
    services.ephemera.kairos._configJson = settingsFormat.generate "kairos.json" cfg.settings;

    services.ephemera.kairos._heraldConfigJson = settingsFormat.generate "config.json" cfg.heraldSettings;

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
        ExecStartPre = "${pkgs.coreutils}/bin/mkdir -p ${dirOf cfg.settings.database_path}";
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
