{ flake }:
{ pkgs, ... }:
let
  ephaPkgs = flake.packages.${pkgs.stdenv.hostPlatform.system};
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
}
