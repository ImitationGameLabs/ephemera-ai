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

    ctlPackage = lib.mkOption {
      type = lib.types.package;
      default = ephaPkgs.epha-ctl;
      description = "The epha-ctl package to use";
    };

    installCtlPackage = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = "Whether to auto-install epha-ctl in home.packages when epha-ai is enabled";
    };

    tmuxPackage = lib.mkOption {
      type = lib.types.package;
      default = pkgs.tmux;
      description = "The tmux package used by epha-ai shell sessions";
    };

    log_level = lib.mkOption {
      type = lib.types.str;
      default = "info";
      description = ''
        Log filter directive passed as RUST_LOG to the service process.
        Accepts a plain level ("info", "debug", "warn") or a comma-separated
        EnvFilter directive for fine-grained per-crate control
        (e.g. "debug,hyper=warn,sqlx=warn").
      '';
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
        max_pinned_tokens = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Maximum token budget for pinned memories";
        };

        total_token_floor = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Total token budget floor - eviction stops at this level (includes all components)";
        };

        total_token_ceiling = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Total token budget ceiling - eviction triggers at this level (includes all components)";
        };

        response_reserve_tokens = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Tokens reserved for LLM response output";
        };

        min_activities = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Minimum number of recent activities to preserve during eviction";
        };
      };

      prompt_append_file = lib.mkOption {
        type = lib.types.nullOr lib.types.path;
        default = null;
        description = ''
          Optional path to a Markdown file appended to the grounding prompt.
          Useful for injecting context-specific grounding (e.g., integration tests).
          Set to a Nix store path, e.g. the flake source path of a prompt file.
        '';
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

    home.packages = lib.mkIf (cfg.enable && cfg.installCtlPackage) [
      (pkgs.writeShellScriptBin "epha-ctl" ''
        export EPHA_CONFIG_DIR=${config.services.ephemera._configDir}
        exec ${cfg.ctlPackage}/bin/epha-ctl "$@"
      '')
    ];

    systemd.user.services.epha-ai = lib.mkIf cfg.enable (
      let
        runScript = pkgs.writeShellScript "epha-ai-run" ''
          export PATH="${lib.makeBinPath [ cfg.tmuxPackage ]}:$PATH"
          exec ${cfg.package}/bin/epha-ai --config-dir ${config.services.ephemera._configDir}/epha-ai
        '';
      in
      {
        Unit = {
          Description = "Ephemera AI Agent";
          After = [
            "network.target"
            "loom.service"
            "agora.service"
          ];
          Requires = [ "loom.service" ];
          # Cap restart storms so repeated failures don't flood LLM providers and trigger abuse defenses.
          StartLimitIntervalSec = "3600";
          StartLimitBurst = "5";
        };

        Service = {
          Environment = [ "RUST_LOG=${cfg.log_level}" ];
          ExecStart = "${runScript}";
          Restart = "on-failure";
          RestartSec = "5";
        };

        Install = {
          WantedBy = [ "default.target" ];
        };
      }
    );
  };
}
