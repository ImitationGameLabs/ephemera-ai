{ flake }:
{
  pkgs,
  lib,
  config,
  ...
}:
let
  ephaPkgs = flake.packages.${pkgs.stdenv.hostPlatform.system};
  ephCfg = config.services.ephemera;
in
{
  imports = [
    (import ./agora.nix { inherit ephaPkgs; })
    (import ./atrium.nix { inherit ephaPkgs; })
    (import ./epha-ai.nix { inherit ephaPkgs; })
    (import ./kairos.nix { inherit ephaPkgs; })
    (import ./loom.nix { inherit ephaPkgs; })
    ./mysql.nix
  ];

  # Unified config derivation - collected from all enabled services
  options.services.ephemera._configDir = lib.mkOption {
    type = lib.types.path;
    internal = true;
    description = "Unified config directory for all ephemera services";
  };

  config.services.ephemera._configDir = pkgs.runCommand "ephemera-ai-config" { } ''
    mkdir -p $out
    ${lib.optionalString ephCfg.agora.enable ''
      mkdir -p $out/agora
      ln -s ${ephCfg.agora._configJson} $out/agora/agora.json
    ''}
    ${lib.optionalString ephCfg.kairos.enable ''
      mkdir -p $out/kairos
      ln -s ${ephCfg.kairos._configJson} $out/kairos/kairos.json
      mkdir -p $out/kairos-herald
      ln -s ${ephCfg.kairos._heraldConfigJson} $out/kairos-herald/config.json
    ''}
    ${lib.optionalString ephCfg.atrium.enable ''
      mkdir -p $out/atrium
      ln -s ${ephCfg.atrium._configJson} $out/atrium/atrium.json
      mkdir -p $out/atrium-herald
      ln -s ${ephCfg.atrium._heraldConfigJson} $out/atrium-herald/config.json
    ''}
    ${lib.optionalString ephCfg.loom.enable ''
      mkdir -p $out/loom
      ln -s ${ephCfg.loom._configJson} $out/loom/loom.json
    ''}
    ${lib.optionalString ephCfg.epha-ai.enable ''
      mkdir -p $out/epha-ai
      ln -s ${ephCfg.epha-ai._configJson} $out/epha-ai/epha-ai.json
    ''}
  '';
}
