{
  pkgs,
  lib,
  common,
}:
let
  inherit (common)
    craneLib
    cargoArtifacts
    src
    gitVersion
    binaryCratePaths
    mkCrateFileset
    ;

  individualCrateArgs = {
    inherit cargoArtifacts;
    inherit src;
    strictDeps = true;

    nativeBuildInputs = with pkgs; [
      pkg-config
    ];

    buildInputs =
      with pkgs;
      [
        openssl
      ]
      ++ lib.optionals pkgs.stdenv.isDarwin [
        pkgs.libiconv
      ];

    version = gitVersion;
    # NB: we disable tests since we'll run them all via cargo-nextest
    doCheck = false;
  };

  # All binary package derivations
  packages = lib.mapAttrs (
    pname: cratepath:
    craneLib.buildPackage (
      individualCrateArgs
      // {
        inherit pname;
        cargoExtraArgs = "-p ${pname}";
        src = mkCrateFileset cratepath;
      }
    )
  ) binaryCratePaths;

  # Meta package combining all core services
  ephemera-ai = pkgs.symlinkJoin {
    name = "ephemera-ai";
    paths = lib.attrValues packages;
  };
in
# Actual packages for flake output
packages
// {
  default = ephemera-ai;
  inherit ephemera-ai;
}
