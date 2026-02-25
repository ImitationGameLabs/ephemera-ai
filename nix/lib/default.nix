# Ephemera AI Nix Library
# Provides configuration schema, validation, and service building functions

{ inputs }:

{
  inherit (import ./config.nix { inherit inputs; })
    validateConfig
    generateAllConfigs;

  inherit (import ./services.nix { inherit inputs; })
    mkServiceWrapper;
}
