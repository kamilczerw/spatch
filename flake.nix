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
    flake-utils.lib.eachSystem
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
            version = "0.1.0";
            src = self;

            cargoLock.lockFile = ./Cargo.lock;

            cargoSha256 = lib.fakeSha256;
          };

          devShells.default = pkgs.mkShell {
            packages = [
              pkgs.rustPlatform.rustc
              pkgs.rustPlatform.cargo
            ]
            ++ lib.optionals;
          };
        }
      );
}
