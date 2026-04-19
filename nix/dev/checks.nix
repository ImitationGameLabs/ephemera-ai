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
    inherit advisory-db;

  # The advisory (RUSTSEC-2023-0071) concerns potential side-channel leakage
  # in the RSA implementation (non-constant-time behavior).
  #
  # Based on our threat model, this is not considered exploitable:
  # - No attacker-controlled high-frequency queries
  # - No exposure of a timing oracle
  # - No shared hardware or co-resident adversaries
  #
  # This is a known limitation of the library rather than a fixable bug, as discussed here:
  # https://github.com/RustCrypto/RSA/issues/19#issuecomment-1822995643
    cargoAuditExtraArgs = "--ignore RUSTSEC-2023-0071";
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
}
