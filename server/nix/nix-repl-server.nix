{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.custom.nix-repl-server;

  nixReplImage = pkgs.dockerTools.buildLayeredImage {
    name = "nix-repl-server";
    tag = "latest";
    contents = [
      cfg.package
      pkgs.nix
      pkgs.bashInteractive
      pkgs.cacert
      pkgs.tini
      pkgs.coreutils
    ];
    config = {
      Entrypoint = [
        "${pkgs.tini}/bin/tini"
        "--"
      ];
      Cmd = [ "${cfg.package}/bin/nix-repl-server" ];
      ExposedPorts = {
        "8080/tcp" = { };
      };
      Env = [
        "NIX_REPL_BIND=0.0.0.0"
        "NIX_CONFIG=experimental-features = nix-command flakes"
      ];
    };
  };
in
{
  options.custom.nix-repl-server = {
    enable = lib.mkEnableOption "nix-repl-server container";

    package = lib.mkOption {
      type = lib.types.package;
      description = "The nix-repl-server package to use.";
    };

    port = lib.mkOption {
      type = lib.types.port;
      default = 8080;
    };
    tokenFile = lib.mkOption {
      type = lib.types.path;
      default = "/etc/nix-repl-server.env";
    };
  };

  config = lib.mkIf cfg.enable {
    virtualisation.podman.enable = true;
    virtualisation.oci-containers.backend = "podman";
    virtualisation.oci-containers.containers.nix-repl-server = {
      image = "nix-repl-server:latest";
      imageFile = nixReplImage;
      ports = [ "127.0.0.1:${toString cfg.port}:8080" ];
      environmentFiles = [ cfg.tokenFile ];
      extraOptions = [
        "--cap-drop=ALL"
        "--security-opt=no-new-privileges"
      ];
    };
  };
}
