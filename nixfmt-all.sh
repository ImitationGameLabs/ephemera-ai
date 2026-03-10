#!/usr/bin/env bash

# Format all Nix files in nix/, templates/, and the root flake.nix

nixfmt $(find nix/ templates/ -name '*.nix') flake.nix
