# Service Wrapper Functions
# Creates service packages with embedded configuration

{ inputs }:

let
  # mkServiceWrapper: system -> attrset -> derivation
  # Usage: mkServiceWrapper system { name, package, configJson, serviceName? }
  mkServiceWrapper =
    system:
    {
      name,
      package,
      configJson,
      serviceName ? name,
    }:
    let
      pkgs = inputs.nixpkgs.legacyPackages.${system};
    in
    pkgs.runCommand "${name}-wrapped"
      {
        nativeBuildInputs = [ pkgs.makeWrapper ];
        meta = {
          description = "${name} service with embedded configuration";
          mainProgram = name;
        };
      }
      ''
        mkdir -p $out/bin
        makeWrapper ${package}/bin/${name} $out/bin/${name} \
          --set EPHEMERA_CONFIG_PATH ${configJson} \
          --set EPHEMERA_SERVICE_NAME ${serviceName}
      '';

  mkSystemdService =
    system:
    {
      name,
      package,
      description ? "${name} service",
    }:
    let
      pkgs = inputs.nixpkgs.legacyPackages.${system};
    in
    pkgs.writeText "${name}.service" ''
      [Unit]
      Description=${description}
      After=network.target

      [Service]
      Type=simple
      ExecStart=${package}/bin/${name}
      Restart=on-failure
      RestartSec=5

      [Install]
      WantedBy=multi-user.target
    '';

in
{
  inherit mkServiceWrapper mkSystemdService;
}
