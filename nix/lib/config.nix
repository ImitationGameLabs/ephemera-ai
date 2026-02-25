# Configuration Validation and Generation
# Uses evalModules for validation with NixOS module system

{ inputs }:

let
  lib = inputs.nixpkgs.lib;
  schema = import ./schema.nix { inherit lib; };

  # Force deep evaluation of config to trigger lazy type errors
  # Returns the validated config if valid, throws detailed error on type errors
  validateConfigStrict = userConfig:
    let
      evaled = lib.evalModules {
        modules = [
          schema
          { config = userConfig; }
        ];
      };
      # Force deep evaluation of all config values to trigger type checking
      # toJSON traverses the entire config tree
      jsonStr = builtins.toJSON evaled.config;
    in builtins.deepSeq jsonStr evaled.config;

  # Generate JSON config file for a specific service
  generateServiceConfig = serviceName: config:
    let
      serviceConfig = config.${serviceName} or (throw "Service ${serviceName} not found in config");
    in
      builtins.toFile "${serviceName}-config.json" (builtins.toJSON serviceConfig);
in
{
  # Validate user config against schema
  # Returns null if valid, throws detailed error on validation failure
  # The error message includes full type information from NixOS module system
  # Example error: "A definition for option `loom.port' is not of type `16 bit unsigned integer'"
  validateConfig = userConfig:
    builtins.seq (validateConfigStrict userConfig) null;

  # Generate all service config files
  generateAllConfigs = config:
    let
      serviceNames = [ "epha-ai" "loom" "atrium" "loom-vector" ];
    in
      lib.genAttrs serviceNames (name: generateServiceConfig name config);
}
