{ config, username, ... }:
{
  # Ephemera AI - Main Agent
  services.ephemera.epha-ai = {
    enable = true;
    log_level = "info";

    settings = {
      llm = {
        base_url = "https://api.deepseek.com";
        model = "deepseek-chat";
        max_turns = 20;
        api_key = "sk-xxx";
      };

      services = {
        loom_url = "http://localhost:3001";
      };

      dormant_tick_interval_ms = 60000;

      agora = {
        url = "http://localhost:3000";
      };

      context = {
        max_pinned_tokens = 10000;
        total_token_floor = 4000;
        total_token_ceiling = 100000;
        response_reserve_tokens = 4096;
        min_activities = 2;
      };

      # Optional: append a Markdown file to the grounding prompt.
      # Set to a Nix store path to inject context-specific grounding
      # (e.g., for integration tests). Leave as null for normal operation.
      prompt_append_file = null;
    };
  };

  # Loom - Memory Service
  services.ephemera.loom = {
    enable = true;
    log_level = "info";
    mysql = "loom-mysql";

    settings = {
      port = 3001;
      mysql = {
        max_connections = 10;
      };
    };
  };

  # Agora - Event Hub
  services.ephemera.agora = {
    enable = true;
    log_level = "info";

    settings = {
      port = 3000;
      database_path = "${config.xdg.dataHome}/ephemera/agora.db";
      heartbeat_check_interval_ms = 5000;
      timeout_ms = 30000;

      retry = {
        base_interval_ms = 5000;
        multiplier = 2;
        max_interval_ms = 300000;
      };
    };
  };

  # Kairos - Time Service
  services.ephemera.kairos = {
    enable = true;
    log_level = "info";

    settings = {
      port = 3003;
      database_path = "${config.xdg.dataHome}/ephemera/kairos.db";
      tick_interval_ms = 1000;
    };

    heraldSettings = {
      kairos_url = "http://localhost:3003";
      agora_url = "http://localhost:3000";
      poll_interval_ms = 1000;
      heartbeat_interval_sec = 30;
    };
  };

  # Atrium - Chat Service
  services.ephemera.atrium = {
    enable = true;
    log_level = "info";
    mysql = "atrium-mysql";

    settings = {
      port = 3002;
    };

    # Auth used by atrium-herald, also serves as atrium-cli default config
    auth = {
      username = username;
      password = "goodluck";
      bio = "";
    };

    herald = {
      agora_url = "http://localhost:3000";
      poll_interval_ms = 1000;
      heartbeat_interval_ms = 30000;
      atrium_heartbeat_interval_ms = 30000;
    };
  };

  # MySQL instances
  services.ephemera.mysql = {
    "loom-mysql" = {
      port = 3306;
      database = "psyche_loom";
      user = "epha";
      password = "123456";
      volume = "loom-mysql-data";
    };

    "atrium-mysql" = {
      port = 3307;
      database = "dialogue_atrium";
      user = "epha";
      password = "123456";
      volume = "atrium-mysql-data";
    };
  };
}
