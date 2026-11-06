{
  description = "JSON Patch that doesn’t suck with collections";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    {
      self,
      nixpkgs,
      flake-utils,
    }:
    (flake-utils.lib.eachSystem
      [
        "x86_64-linux"
        "aarch64-linux"
        "x86_64-darwin"
        "aarch64-darwin"
      ]
      (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          lib = pkgs.lib;
        in
        {
          packages.default = pkgs.rustPlatform.buildRustPackage rec {
            pname = "spatch";
            version = (lib.importTOML ./Cargo.toml).package.version;
            src = self;

            cargoLock.lockFile = ./Cargo.lock;
          };

          apps.default = flake-utils.lib.mkApp { drv = self.packages.${system}.default; };

          devShells.default = pkgs.mkShell {
            packages = [
              pkgs.rustc
              pkgs.cargo
            ];
          };
        }
      )
    )
    // {
      overlays.default = final: prev: {
        spatch = self.packages.${final.stdenv.hostPlatform.system}.default;
      };
    };
}
