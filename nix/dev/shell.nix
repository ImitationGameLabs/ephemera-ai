{
  pkgs,
  common,
  checks,
}:
let
  inherit (common) craneLib;
in
craneLib.devShell {
  inherit checks;

  # Extra inputs can be added here; cargo and rustc are provided by default.
  packages = with pkgs; [
    # Rust
    cargo-hakari
    rust-analyzer

    # Nix
    nixd
    nixfmt
    statix

    # Frontend
    # NOTE: Using nodejs_25 is intentional. Development environment consistency is guaranteed
    # through nix flake. If compatibility issues arise in the future, we can revisit this.
    nodejs_25
    pnpm

    # A terminal multiplexer (ephemera-ai uses it for shell session management)
    tmux

    # TOML toolkit (linter, formatter)
    taplo

    # Temporary workaround for copilot-cli direnv integration bug
    # See: https://github.com/github/copilot-cli/issues/731
    # TODO: Remove once the upstream issue is resolved
    bashInteractive
  ];
}
