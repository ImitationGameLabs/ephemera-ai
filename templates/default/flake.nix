# Ephemera AI Deployment Template
# Initialize with: nix flake init -t github:ImitationGameLabs/ephemera-ai

{
  description = "Ephemera AI deployment configuration";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    ephemera-ai.url = "github:ImitationGameLabs/ephemera-ai";
  };

  outputs =
    inputs@{ flake-parts, ephemera-ai, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      perSystem =
        {
          pkgs,
          system,
          lib,
          ...
        }:
        let
          userConfig = import ./config.nix;

          ephaLib = ephemera-ai.lib;

          configErrors = ephaLib.validateConfig userConfig;
          validatedConfig =
            if configErrors != null then
              throw "Configuration validation failed:\n${lib.concatStringsSep "\n" configErrors}"
            else
              userConfig;

          configFiles = ephaLib.generateAllConfigs validatedConfig;

          basePackages = ephemera-ai.packages.${system};

          mkServiceWrapper' =
            name:
            ephaLib.mkServiceWrapper system {
              name = name;
              package = basePackages.${name};
              configJson = configFiles.${name};
            };
        in
        {
          packages = rec {
            epha-ai-wrapped = mkServiceWrapper' "epha-ai";
            loom-wrapped = mkServiceWrapper' "loom";
            atrium-wrapped = mkServiceWrapper' "atrium";
            default = epha-ai-wrapped;
          };

          checks.config-validation = pkgs.runCommand "config-validation" { } ''
            echo "Configuration validated successfully"
            touch $out
          '';
        };
    };
}
