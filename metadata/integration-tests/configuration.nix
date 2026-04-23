{ pkgs, ... }:
{
  # Why isNspawnContainer (not isContainer):
  #   1. --config-file auto-injects container boot settings; --flake uses our
  #      own config, so we must set this explicitly.
  #   2. nixos-container uses systemd-nspawn.  isNspawnContainer (which implies
  #      isContainer) is the correct, more specific option.
  boot.isNspawnContainer = true;

  system.stateVersion = "25.11";

  nixpkgs.config.allowUnfree = true;

  # Enable flakes inside the container
  nix.settings.experimental-features = [
    "nix-command"
    "flakes"
  ];

  # User for home-manager deployment
  users.users.ephemera = {
    isNormalUser = true;
    uid = 1000;
    home = "/home/ephemera";
    createHome = true;
    shell = "${pkgs.bash}/bin/bash";
    linger = true;
  };

  environment.variables = {
    XDG_RUNTIME_DIR = "/run/user/1000";
  };

  # MySQL for integration test (shared server for loom + atrium)
  services.mysql = {
    enable = true;
    package = pkgs.mysql84;
    initialScript = pkgs.writeText "mysql-init.sql" ''
      CREATE DATABASE IF NOT EXISTS psyche_loom;
      CREATE DATABASE IF NOT EXISTS dialogue_atrium;
      CREATE USER IF NOT EXISTS 'epha'@'localhost' IDENTIFIED BY 'integration-test-pass';
      GRANT ALL PRIVILEGES ON psyche_loom.* TO 'epha'@'localhost';
      GRANT ALL PRIVILEGES ON dialogue_atrium.* TO 'epha'@'localhost';
      FLUSH PRIVILEGES;
    '';
  };

  environment.systemPackages = with pkgs; [
    git
    curl
    home-manager
  ];
}
