{
  pkgs,
  lib,
  inputs,
  # Project root path, must be passed from flake.nix (not ./.) because:
  # - Nix paths are evaluated at definition site
  # - If we use ./. here, it would resolve to nix/ directory, not project root
  # - Passing from flake.nix ensures ./. resolves to the correct location
  root,
}:
let
  # NB: we don't need to overlay our custom toolchain for the *entire*
  # pkgs (which would require rebuidling anything else which uses rust).
  # Instead, we just want to update the scope that crane will use by appending
  # our specific toolchain there.
  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain (
    p:
    p.rust-bin.stable.latest.default.override {
      extensions = [ "rust-src" ];
      # targets = [ "wasm32-wasip1" ];
    }
  );

  src = craneLib.cleanCargoSource root;

  # Use git shortRev as version, fallback to "dirty" if working tree is dirty
  gitVersion = inputs.self.shortRev or "dirty";

  # Common arguments can be set here to avoid repeating them later
  commonArgs = {
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
        # Additional darwin specific inputs can be set here
        pkgs.libiconv
      ];
  };

  # Build *just* the cargo dependencies (of the entire workspace),
  # so we can reuse all of that work (e.g. via cachix) when running in CI
  cargoArtifacts = craneLib.buildDepsOnly commonArgs;

  # mapToAbsolute is a function that converts relative crate paths to absolute paths.
  # Takes an attrset like { epha-ai = "crates/epha-ai"; }
  # and returns { epha-ai = /absolute/path/to/crates/epha-ai; }
  mapToAbsolute = lib.mapAttrs (_: path: root + "/${path}");

  # Binary crates that produce executables (need to be built individually)
  binaryCratePaths = mapToAbsolute {
    epha-ai = "crates/epha-ai";
    epha-ctl = "crates/epha-ctl";
    agora = "crates/agora";
    kairos = "crates/chronikos/kairos";
    kairos-cli = "crates/chronikos/kairos-cli";
    kairos-herald = "crates/chronikos/kairos-herald";
    loom = "crates/psyche/loom";
    atrium = "crates/dialogue/atrium";
    atrium-cli = "crates/dialogue/atrium-cli";
    atrium-herald = "crates/dialogue/atrium-herald";
  };

  # Library-only crates (only needed for fileset dependencies, not built separately)
  libraryCratePaths = mapToAbsolute {
    agora-common = "crates/agora-common";
    agora-client = "crates/agora-client";
    kairos-common = "crates/chronikos/kairos-common";
    kairos-client = "crates/chronikos/kairos-client";
    loom-common = "crates/psyche/loom-common";
    loom-client = "crates/psyche/loom-client";
    atrium-common = "crates/dialogue/atrium-common";
    atrium-client = "crates/dialogue/atrium-client";
  };

  # mkCrateSources is a function that converts a crate paths attrset to a list of filesets.
  # Used to gather all workspace crate sources for dependency tracking.
  mkCrateSources =
    cratePaths:
    lib.mapAttrsToList (_: cratepath: craneLib.fileset.commonCargoSources cratepath) cratePaths;

  # All workspace crates combined (for fileset generation)
  allCratePaths = binaryCratePaths // libraryCratePaths;

  # mkCrateFileset is a function that creates a fileset for building a specific crate.
  # Includes workspace-level files (Cargo.toml, Cargo.lock) and all workspace crate
  # sources. All members are needed because cargo validates their existence even
  # when building a single crate with -p.
  mkCrateFileset =
    cratepath:
    lib.fileset.toSource {
      inherit root;
      fileset = lib.fileset.unions (
        [
          (root + "/Cargo.toml")
          (root + "/Cargo.lock")
        ]
        ++ mkCrateSources allCratePaths
        ++ [ (craneLib.fileset.commonCargoSources cratepath) ]
      );
    };
in
{
  inherit
    craneLib
    src
    commonArgs
    cargoArtifacts
    gitVersion
    binaryCratePaths
    libraryCratePaths
    mkCrateFileset
    ;
}
