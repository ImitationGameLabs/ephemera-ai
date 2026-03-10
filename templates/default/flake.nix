{
  description = "Home Manager Configuration for Ephemera AI";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.11";
    nixpkgs-unstable.url = "github:nixos/nixpkgs/nixos-unstable";

    home-manager = {
      url = "github:nix-community/home-manager/release-25.11";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    ephemera-ai = {
      url = "github:ImitationGameLabs/ephemera-ai";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      nixpkgs-unstable,
      home-manager,
      ephemera-ai,
      ...
    }@inputs:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
    in
    {
      inherit inputs;

      homeConfigurations.simplex = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;

        extraSpecialArgs = {
          inherit inputs;
        };

        modules = [
          {
            nixpkgs.overlays = [
              # NixOS unstable channel overlay
              (final: prev: {
                unstable = import nixpkgs-unstable {
                  inherit (final) config;
                  inherit (final.stdenv.hostPlatform) system;
                };
              })
            ];
          }

          # Import ephemera-ai home-manager modules
          ephemera-ai.homeManagerModules.default

          ./home.nix
          ./env.nix
          ./ephemera-ai.nix
        ];
      };
    };
}
