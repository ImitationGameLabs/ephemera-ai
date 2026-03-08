# Ephemera AI Configuration
#
# All fields are required. There are no defaults.
# Missing or invalid fields will cause build errors.

{
  # Main AI Agent Service
  epha-ai = {
    # LLM Configuration (OpenAI-compatible API)
    # Model must support function calling for tool usage
    llm = {
      base_url = "https://api.deepseek.com";
      model = "deepseek-chat";
      api_key = "sk-xxx";
    };

    # Service URLs (used by epha-ai to connect to other services)
    services = {
      loom_url = "http://localhost:3001";
      atrium_url = "http://localhost:3002";
      loom_vector_url = "http://localhost:3003";
    };

    # Atrium authentication credentials
    atrium_auth = {
      username = "admin";
      password = "password";
    };

    # Context management configuration
    context = {
      max_pinned_count = 5;
    };
  };

  # Memory Service (MySQL-based storage)
  loom = {
    mysql_url = "mysql://epha:123456@localhost:3306/psyche_loom";
    port = 3001;
  };

  # Chat Service (MySQL-based storage)
  atrium = {
    mysql_url = "mysql://epha:123456@localhost:3307/dialogue_atrium";
    port = 3002;
  };

  # Vector Search Service
  loom-vector = {
    port = 3003;
    qdrant_url = "http://localhost:6334";

    # Embedding Model Configuration (OpenAI-compatible API)
    # Note: Do NOT include /embeddings in the URL - it will be added automatically
    embedding = {
      base_url = "https://api.openai.com/v1";
      api_key = "sk-xxx";
      model = "text-embedding-3-small";
      dimensions = 1536;
    };
  };
}
