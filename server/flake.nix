{
  description = "mdbook-nix-repl server module";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
  };

  outputs =
    { self, nixpkgs }:
    let
      supportedSystems = [
        "x86_64-linux"
        "aarch64-linux"
      ];
      forAllSystems = nixpkgs.lib.genAttrs supportedSystems;
    in
    {
      # 1. Output the Package
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          nix-repl-server = pkgs.callPackage ./nix/server-pkg.nix {
            # Point to the folder containing Cargo.toml
            src = ./.;
          };
          default = self.packages.${system}.nix-repl-server;
        }
      );

      # 2. Output the Module
      nixosModules.default =
        {
          config,
          lib,
          pkgs,
          ...
        }:
        {
          imports = [ ./nix/nix-repl-server.nix ];

          # Automatically inject the package from this flake
          custom.nix-repl-server.package = self.packages.${pkgs.system}.nix-repl-server;
        };
    };
}
