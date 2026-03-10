{ config, ... }:
{
  # Ephemera AI - Main Agent
  services.ephemera.epha-ai = {
    enable = true;

    llm = {
      baseUrl = "https://api.deepseek.com";
      model = "deepseek-chat";
      maxTurns = 10;

      # Replace with your API key
      apiKey = "sk-xxx";
    };

    services = {
      loomUrl = "http://localhost:3001";
      atriumUrl = "http://localhost:3002";
      agoraUrl = "http://localhost:3000";
    };

    dormantTickIntervalMs = 60000;

    context = {
      maxPinnedCount = 5;
    };
  };

  # Loom - Memory Service
  services.ephemera.loom = {
    enable = true;
    port = 3001;
    mysql = "loom-mysql";
    mysqlMaxConnections = null;
    agoraUrl = "http://localhost:3000";
  };

  # Agora - Event Hub
  services.ephemera.agora = {
    enable = true;
    port = 3000;
    # Using XDG data directory: ~/.local/share/ephemera/agora.db
    databasePath = "${config.xdg.dataHome}/ephemera/agora.db";
    heartbeatCheckIntervalMs = 5000;
    timeoutMs = 30000;

    retry = {
      baseIntervalMs = 5000;
      multiplier = 2;
      maxIntervalMs = 300000;
    };
  };

  # Kairos - Time Service
  services.ephemera.kairos = {
    enable = true;
    port = 3003;
    # Using XDG data directory: ~/.local/share/ephemera/kairos.db
    databasePath = "${config.xdg.dataHome}/ephemera/kairos.db";
    tickIntervalMs = 1000;
    agoraUrl = "http://localhost:3000";
    heraldPollIntervalMs = 1000;
    heraldHeartbeatIntervalSec = 30;
  };

  # Atrium - Chat Service
  services.ephemera.atrium = {
    enable = true;
    port = 3002;
    mysql = "atrium-mysql";
    agoraUrl = "http://localhost:3000";

    heraldAuth = {
      username = "epha";
      password = "your-secure-password";
      bio = null;
    };

    heraldPollIntervalMs = 1000;
    heraldHeartbeatIntervalMs = 30000;
    heraldAtriumHeartbeatIntervalMs = 30000;
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
