{
  config,
  lib,
  pkgs,
  ...
}:

let
  cfg = config.services.ephemera.mysql;
in
{
  options.services.ephemera.mysql = lib.mkOption {
    type = lib.types.attrsOf (
      lib.types.submodule {
        options = {
          port = lib.mkOption {
            type = lib.types.port;
            description = "MySQL port";
          };

          database = lib.mkOption {
            type = lib.types.str;
            description = "Database name";
          };

          user = lib.mkOption {
            type = lib.types.str;
            description = "Database user";
          };

          password = lib.mkOption {
            type = lib.types.str;
            description = "Database password";
          };

          image = lib.mkOption {
            type = lib.types.str;
            default = "docker.io/mysql:8.0";
            description = "MySQL container image";
          };

          volume = lib.mkOption {
            type = lib.types.str;
            description = "Podman named volume for persistent data";
          };
        };
      }
    );
    default = { };
    description = "MySQL instances managed via podman";
  };

  config = lib.mkIf (cfg != { }) {
    home.packages = [ pkgs.podman ];

    systemd.user.services = lib.mapAttrs' (name: mysqlCfg: {
      name = "${name}";
      value =
        let
          # Use --env-file to avoid exposing passwords in process listings (ps aux, /proc/<pid>/cmdline)
          #
          # FIXME: envFile is stored in /nix/store with 0644 permissions (world-readable).
          # Current trade-offs:
          # 1. The file path contains a hash - attackers must know the exact path to read it
          # 2. Compared to CLI args, this avoids plaintext exposure in process lists and systemd logs
          # 3. Consider using sops-nix or agenix for proper secrets management in the future
          envFile = pkgs.writeText "mysql-env-${name}" ''
            MYSQL_ROOT_PASSWORD=${mysqlCfg.password}
            MYSQL_DATABASE=${mysqlCfg.database}
            MYSQL_USER=${mysqlCfg.user}
            MYSQL_PASSWORD=${mysqlCfg.password}
          '';
        in
        {
          Unit = {
            Description = "MySQL container for ${name}";
            After = [ "podman.socket" ];
            Requires = [ "podman.socket" ];
            # Allow fast recovery during startup dependency races, but still bound restart loops.
            StartLimitIntervalSec = "300";
            StartLimitBurst = "20";
          };

          Service = {
            ExecStart = "${pkgs.podman}/bin/podman run --rm --name ${name} -p ${toString mysqlCfg.port}:3306 -v ${mysqlCfg.volume}:/var/lib/mysql --env-file ${envFile} ${mysqlCfg.image}";
            ExecStop = "${pkgs.podman}/bin/podman stop -t 10 ${name}";
            Restart = "on-failure";
            RestartSec = "3";
          };

          Install = {
            WantedBy = [ "default.target" ];
          };
        };
    }) cfg;
  };
}
