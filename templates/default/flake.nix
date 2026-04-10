{
  description = "Home Manager Configuration for Ephemera AI";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";

    home-manager = {
      url = "github:nix-community/home-manager/release-25.11";
      # inputs.nixpkgs.follows = "nixpkgs";
    };

    ephemera-ai = {
      url = "github:ImitationGameLabs/ephemera-ai";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    {
      nixpkgs,
      home-manager,
      ephemera-ai,
      ...
    }@inputs:
    let
      system = "x86_64-linux";
      pkgs = nixpkgs.legacyPackages.${system};
      username = "ephemera";
    in
    {
      homeConfigurations.${username} = home-manager.lib.homeManagerConfiguration {
        inherit pkgs;

        extraSpecialArgs = {
          inherit inputs username;
        };

        modules = [
          ephemera-ai.homeManagerModules.default
          ./home.nix
          ./env.nix
          ./ephemera-ai.nix
        ];
      };
    };
}
