{
  pkgs,
  common,
  advisory-db,
  # Individual crate derivations
  ephaPkgs,
}:
let
  inherit (common)
    craneLib
    src
    commonArgs
    cargoArtifacts
    ;
in
# Include all binary crates in checks (excludes meta packages: default, ephemera-ai)
(pkgs.lib.removeAttrs ephaPkgs [
  "default"
  "ephemera-ai"
])
// {
  # Run clippy (and deny all warnings) on the workspace source,
  # again, reusing the dependency artifacts from above.
  ephemera-ai-clippy = craneLib.cargoClippy (
    commonArgs
    // {
      inherit cargoArtifacts;
      cargoClippyExtraArgs = "--all-targets -- --deny warnings";
    }
  );

  ephemera-ai-doc = craneLib.cargoDoc (
    commonArgs
    // {
      inherit cargoArtifacts;
      env.RUSTDOCFLAGS = "--deny warnings";
    }
  );

  # Check formatting
  ephemera-ai-fmt = craneLib.cargoFmt {
    inherit src;
  };

  ephemera-ai-toml-fmt = craneLib.taploFmt {
    src = pkgs.lib.sources.sourceFilesBySuffices src [ ".toml" ];
  };

  # Audit dependencies
  ephemera-ai-audit = craneLib.cargoAudit {
    inherit src;
    advisory-db = advisory-db;
  };

  # Audit licenses
  ephemera-ai-deny = craneLib.cargoDeny {
    inherit src;
  };

  # Run tests with cargo-nextest
  ephemera-ai-nextest = craneLib.cargoNextest (
    commonArgs
    // {
      inherit cargoArtifacts;
      partitions = 1;
      partitionType = "count";
      cargoNextestPartitionsExtraArgs = "--no-tests=pass";
    }
  );

  # Ensure that cargo-hakari is up to date
  ephemera-ai-hakari = craneLib.mkCargoDerivation {
    inherit src;
    pname = "ephemera-ai-hakari";
    cargoArtifacts = null;
    doInstallCargoArtifacts = false;

    buildPhaseCargoCommand = ''
      cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
      cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
      cargo hakari verify
    '';

    nativeBuildInputs = [
      pkgs.cargo-hakari
    ];
  };
}
