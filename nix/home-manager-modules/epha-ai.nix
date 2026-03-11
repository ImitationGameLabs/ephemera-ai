{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.epha-ai;
  settingsFormat = pkgs.formats.json { };
in
{
  options.services.ephemera.epha-ai = {
    enable = lib.mkEnableOption "ephemera-ai main agent service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.epha-ai;
      description = "The epha-ai package to use";
    };

    settings = {
      llm = {
        base_url = lib.mkOption {
          type = lib.types.str;
          description = "LLM API base URL";
        };

        model = lib.mkOption {
          type = lib.types.str;
          description = "LLM model name";
        };

        api_key = lib.mkOption {
          type = lib.types.str;
          description = "LLM API key";
        };

        max_turns = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Maximum number of tool call iterations per cognitive cycle";
        };
      };

      services = {
        loom_url = lib.mkOption {
          type = lib.types.str;
          description = "Loom service URL";
        };
      };

      dormant_tick_interval_ms = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Tick interval in Dormant state (ms)";
      };

      agora = {
        url = lib.mkOption {
          type = lib.types.str;
          description = "Agora service URL";
        };
      };

      context = {
        max_pinned_count = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Maximum number of pinned content items in context";
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
    services.ephemera.epha-ai._configJson = settingsFormat.generate "epha-ai.json" cfg.settings;

    systemd.user.services.epha-ai = lib.mkIf cfg.enable {
      Unit = {
        Description = "Ephemera AI Agent";
        After = [
          "network.target"
          "loom.service"
          "agora.service"
        ];
        Requires = [ "loom.service" ];
      };

      Service = {
        ExecStart = "${cfg.package}/bin/epha-ai --config-dir ${config.services.ephemera._configDir}/epha-ai";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
