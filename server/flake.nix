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
      # 1. Output the Package (so people can build it standalone if they want)
      packages = forAllSystems (
        system:
        let
          pkgs = nixpkgs.legacyPackages.${system};
        in
        {
          nix-repl-server = pkgs.callPackage ./nix/server-pkg.nix {
            # Point to the folder containing Cargo.toml inside your repo
            src = ./.;
          };
          default = self.packages.${system}.nix-repl-server;
        }
      );

      # 2. Output the Module (this is what users import)
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
          # This connects the module to the source code without the user doing anything
          custom.nix-repl-server.package = self.packages.${pkgs.system}.nix-repl-server;
        };
    };
}
