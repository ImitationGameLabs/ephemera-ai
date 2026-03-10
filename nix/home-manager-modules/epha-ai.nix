{ ephaPkgs }:
{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.epha-ai;

  # Generate JSON config file for epha-ai
  ephaAiConfig = pkgs.writeText "epha-ai.json" (
    builtins.toJSON {
      llm = {
        base_url = cfg.llm.baseUrl;
        model = cfg.llm.model;
        api_key = cfg.llm.apiKey;
        max_turns = cfg.llm.maxTurns;
      };
      services = {
        loom_url = cfg.services.loomUrl;
      };
      dormant_tick_interval_ms = cfg.dormantTickIntervalMs;
      agora = {
        url = cfg.services.agoraUrl;
      };
      context = {
        max_pinned_count = cfg.context.maxPinnedCount;
      };
    }
  );

  # Create config directory with the JSON file
  configDir = pkgs.runCommand "epha-ai-config" { } ''
    mkdir -p $out
    ln -s ${ephaAiConfig} $out/epha-ai.json
  '';
in
{
  options.services.ephemera.epha-ai = {
    enable = lib.mkEnableOption "ephemera-ai main agent service";

    package = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.epha-ai;
      description = "The epha-ai package to use";
    };

    llm = {
      baseUrl = lib.mkOption {
        type = lib.types.str;
        description = "LLM API base URL";
      };

      model = lib.mkOption {
        type = lib.types.str;
        description = "LLM model name";
      };

      apiKey = lib.mkOption {
        type = lib.types.str;
        description = "LLM API key";
      };

      maxTurns = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Maximum number of tool call iterations per cognitive cycle";
      };
    };

    services = {
      loomUrl = lib.mkOption {
        type = lib.types.str;
        description = "Loom service URL";
      };

      atriumUrl = lib.mkOption {
        type = lib.types.str;
        description = "Atrium service URL";
      };

      agoraUrl = lib.mkOption {
        type = lib.types.str;
        description = "Agora service URL";
      };
    };

    dormantTickIntervalMs = lib.mkOption {
      type = lib.types.ints.positive;
      description = "Tick interval in Dormant state (ms)";
    };

    context = {
      maxPinnedCount = lib.mkOption {
        type = lib.types.ints.positive;
        description = "Maximum number of pinned content items in context";
      };
    };
  };

  config = lib.mkIf cfg.enable {
    systemd.user.services.epha-ai = {
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
        ExecStart = "${cfg.package}/bin/epha-ai --config-dir ${configDir}";
        Restart = "on-failure";
        RestartSec = "5";
      };

      Install = {
        WantedBy = [ "default.target" ];
      };
    };
  };
}
