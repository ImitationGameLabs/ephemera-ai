# Configuration Schema Definition
# Uses NixOS module system for type checking and documentation

{ lib, ... }:

{
  options = {
    epha-ai = {
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
      };
      services = {
        loom_url = lib.mkOption {
          type = lib.types.str;
          description = "Loom service URL";
        };
        atrium_url = lib.mkOption {
          type = lib.types.str;
          description = "Atrium service URL";
        };
        loom_vector_url = lib.mkOption {
          type = lib.types.str;
          description = "Loom vector service URL";
        };
      };
      atrium_auth = {
        username = lib.mkOption {
          type = lib.types.str;
          description = "Atrium authentication username";
        };
        password = lib.mkOption {
          type = lib.types.str;
          description = "Atrium authentication password";
        };
      };
      context = {
        max_pinned_count = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Maximum number of pinned content items in context";
        };
      };
    };

    loom = {
      mysql_url = lib.mkOption {
        type = lib.types.str;
        description = "MySQL connection URL for Loom";
      };
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port number for Loom service";
      };
    };

    atrium = {
      mysql_url = lib.mkOption {
        type = lib.types.str;
        description = "MySQL connection URL for Atrium";
      };
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port number for Atrium service";
      };
    };

    loom-vector = {
      port = lib.mkOption {
        type = lib.types.port;
        description = "Port number for Loom Vector service";
      };
      qdrant_url = lib.mkOption {
        type = lib.types.str;
        description = "Qdrant vector database URL";
      };
      embedding = {
        base_url = lib.mkOption {
          type = lib.types.str;
          description = "Embedding API base URL";
        };
        api_key = lib.mkOption {
          type = lib.types.str;
          description = "Embedding API key";
        };
        model = lib.mkOption {
          type = lib.types.str;
          description = "Embedding model name";
        };
        dimensions = lib.mkOption {
          type = lib.types.ints.positive;
          description = "Embedding vector dimensions";
        };
      };
    };
  };
}
