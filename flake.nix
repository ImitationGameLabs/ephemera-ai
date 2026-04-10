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
    inputs@{ self, flake-parts, ... }:
    flake-parts.lib.mkFlake { inherit inputs; } {
      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ];

      # Flake-level outputs
      flake = {
        # Export home-manager modules
        homeManagerModules.default = import ./nix/home-manager-modules { flake = self; };

        # Export templates for user initialization
        templates = {
          default = {
            path = ./templates/default;
            description = "Ephemera AI deployment configuration (home-manager)";
          };
        };
      };

      perSystem =
        { system, lib, ... }:
        let
          pkgs = import inputs.nixpkgs {
            inherit system;
            overlays = [ (import inputs.rust-overlay) ];
          };

          root = ./.;

          common = import ./nix/common.nix {
            inherit
              pkgs
              lib
              inputs
              root
              ;
          };

          packages = import ./nix/packages.nix {
            inherit pkgs lib common;
          };

          checks = import ./nix/dev/checks.nix {
            inherit pkgs common;
            inherit (inputs) advisory-db;
            ephaPkgs = packages;
          };
        in
        {
          inherit packages checks;

          devShells.default = import ./nix/dev/shell.nix {
            inherit pkgs common checks;
          };
        };
    };
}
