{
  description = "Ephemera AI - AI system with long-term memory and meta-cognition";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    crane.url = "github:ipetkov/crane";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs =
    inputs@{ flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      # Flake-level outputs (lib, templates)
      flake = {
        # Export nix library for configuration and service building
        lib = import ./nix/lib { inherit inputs; };

        # Export templates for user initialization
        templates.default = {
          path = ./templates/default;
          description = "Ephemera AI deployment configuration";
        };
      };

      perSystem =
        {
          self',
          pkgs,
          lib,
          ...
        }:
        let
          pkgs' = pkgs.extend (import inputs.rust-overlay);

          craneLib = inputs.crane.mkLib pkgs';
          src = craneLib.cleanCargoSource ./.;

          # Common arguments can be set here to avoid repeating them later
          commonArgs = {
            inherit src;
            strictDeps = true;

            nativeBuildInputs = with pkgs'; [
              pkg-config
            ];

            buildInputs =
              with pkgs';
              [
                openssl
              ]

              ++ lib.optionals pkgs'.stdenv.isDarwin [
                # Additional darwin specific inputs can be set here
                pkgs'.libiconv
              ];

            # Additional environment variables can be set directly
            # MY_CUSTOM_VAR = "some value";
          };

          # Build *just* the cargo dependencies (of the entire workspace),
          # so we can reuse all of that work (e.g. via cachix) when running in CI
          # It is *highly* recommended to use something like cargo-hakari to avoid
          # cache misses when building individual top-level-crates
          cargoArtifacts = craneLib.buildDepsOnly commonArgs;

          individualCrateArgs = commonArgs // {
            inherit cargoArtifacts;
            inherit (craneLib.crateNameFromCargoToml { inherit src; }) version;
            # NB: we disable tests since we'll run them all via cargo-nextest
            doCheck = false;
          };

          fileSetForCrate =
            crate:
            lib.fileset.toSource {
              root = ./.;
              fileset = lib.fileset.unions [
                ./Cargo.toml
                ./Cargo.lock
                (craneLib.fileset.commonCargoSources ./dialogue/atrium)
                (craneLib.fileset.commonCargoSources ./dialogue/atrium-cli)
                (craneLib.fileset.commonCargoSources ./dialogue/atrium-client)
                (craneLib.fileset.commonCargoSources ./psyche/loom)
                (craneLib.fileset.commonCargoSources ./psyche/loom-client)
                (craneLib.fileset.commonCargoSources ./epha-agent)
                (craneLib.fileset.commonCargoSources ./epha-ai)
                (craneLib.fileset.commonCargoSources ./epha-boot)
                # (craneLib.fileset.commonCargoSources ./crates/my-common)
                # (craneLib.fileset.commonCargoSources ./crates/my-workspace-hack)
                (craneLib.fileset.commonCargoSources crate)
              ];
            };

          # Build the top-level crates of the workspace as individual derivations.
          # This allows consumers to only depend on (and build) only what they need.
          # Though it is possible to build the entire workspace as a single derivation,
          # so this is left up to you on how to organize things
          #
          # Note that the cargo workspace must define `workspace.members` using wildcards,
          # otherwise, omitting a crate (like we do below) will result in errors since
          # cargo won't be able to find the sources for all members.
          epha-ai = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "epha-ai";
              cargoExtraArgs = "-p epha-ai";
              src = fileSetForCrate ./epha-ai;
            }
          );
          loom = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "loom";
              cargoExtraArgs = "-p loom";
              src = fileSetForCrate ./psyche/loom;
            }
          );

          epha-boot = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "epha-boot";
              cargoExtraArgs = "-p epha-boot";
              src = fileSetForCrate ./epha-boot;
            }
          );

          atrium = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "atrium";
              cargoExtraArgs = "-p atrium";
              src = fileSetForCrate ./dialogue/atrium;
            }
          );

          atrium-cli = craneLib.buildPackage (
            individualCrateArgs
            // {
              pname = "atrium-cli";
              cargoExtraArgs = "-p atrium-cli";
              src = fileSetForCrate ./dialogue/atrium-cli;
            }
          );

          # Meta package combining core services (epha-ai, loom, atrium, atrium-cli)
          ephemera-ai = pkgs'.symlinkJoin {
            name = "ephemera-ai";
            paths = [
              epha-ai
              loom
              atrium
              atrium-cli
            ];
          };
        in
        {
          packages = {
            default = ephemera-ai;
            inherit
              epha-ai
              loom
              atrium
              atrium-cli
              epha-boot
              ephemera-ai
              ;
          };

          apps = {
            epha-ai = {
              type = "app";
              program = "${epha-ai}/bin/epha-ai";
            };
            loom = {
              type = "app";
              program = "${loom}/bin/loom";
            };
            epha-boot = {
              type = "app";
              program = "${epha-boot}/bin/epha-boot";
            };
            atrium = {
              type = "app";
              program = "${atrium}/bin/atrium";
            };
            atrium-cli = {
              type = "app";
              program = "${atrium-cli}/bin/atrium-cli";
            };
          };

          devShells.default = craneLib.devShell {
            # Inherit inputs from checks.
            checks = self'.checks;

            # Additional dev-shell environment variables can be set directly
            # MY_CUSTOM_DEVELOPMENT_VAR = "something else";

            # Extra inputs can be added here; cargo and rustc are provided by default.
            packages = [
              pkgs'.cargo-hakari
            ];
          };

          checks = {
            # Build the crates as part of `nix flake check` for convenience
            inherit
              epha-ai
              loom
              epha-boot
              atrium
              atrium-cli
              ;

            # Run clippy (and deny all warnings) on the workspace source,
            # again, reusing the dependency artifacts from above.
            #
            # Note that this is done as a separate derivation so that
            # we can block the CI if there are issues here, but not
            # prevent downstream consumers from building our crate by itself.
            my-workspace-clippy = craneLib.cargoClippy (
              commonArgs
              // {
                inherit cargoArtifacts;
                cargoClippyExtraArgs = "--all-targets -- --deny warnings";
              }
            );

            my-workspace-doc = craneLib.cargoDoc (
              commonArgs
              // {
                inherit cargoArtifacts;
                # This can be commented out or tweaked as necessary, e.g. set to
                # `--deny rustdoc::broken-intra-doc-links` to only enforce that lint
                env.RUSTDOCFLAGS = "--deny warnings";
              }
            );

            # Check formatting
            my-workspace-fmt = craneLib.cargoFmt {
              inherit src;
            };

            my-workspace-toml-fmt = craneLib.taploFmt {
              src = pkgs'.lib.sources.sourceFilesBySuffices src [ ".toml" ];
              # taplo arguments can be further customized below as needed
              # taploExtraArgs = "--config ./taplo.toml";
            };

            # Audit dependencies
            my-workspace-audit = craneLib.cargoAudit {
              inherit src;
              advisory-db = inputs.advisory-db;
            };

            # Audit licenses
            my-workspace-deny = craneLib.cargoDeny {
              inherit src;
            };

            # Run tests with cargo-nextest
            # Consider setting `doCheck = false` on other crate derivations
            # if you do not want the tests to run twice
            my-workspace-nextest = craneLib.cargoNextest (
              commonArgs
              // {
                inherit cargoArtifacts;
                partitions = 1;
                partitionType = "count";
                cargoNextestPartitionsExtraArgs = "--no-tests=pass";
              }
            );

            # Ensure that cargo-hakari is up to date
            my-workspace-hakari = craneLib.mkCargoDerivation {
              inherit src;
              pname = "my-workspace-hakari";
              cargoArtifacts = null;
              doInstallCargoArtifacts = false;

              buildPhaseCargoCommand = ''
                cargo hakari generate --diff  # workspace-hack Cargo.toml is up-to-date
                cargo hakari manage-deps --dry-run  # all workspace crates depend on workspace-hack
                cargo hakari verify
              '';

              nativeBuildInputs = [
                pkgs'.cargo-hakari
              ];
            };
          };
        };
    };
}
