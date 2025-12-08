{
  description = "Development environment for Spatch projects";

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
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs {
          inherit system;

          config.allowUnfree = true;
        };
      in
      {
        devShell = pkgs.mkShell {
          buildInputs = with pkgs; [
            gnupg
            gh
            just # Task runner (evolution of make)

            figlet # Print the welcome message as ascii art
            lolcat # Make the ascii art colorful

            cargo-watch # Automatically compile the project when a file changes, usage: `cargo watch -x run`
            cargo-nextest # Test runner
            cargo-chef # Build dependendies to speed up docker build
            cargo-deny
            cargo-nextest # Cargo test runner

            rust-analyzer
            rustup
          ];

          shellHook = ''
            export CARGO_HOME="$(pwd)/.local/cargo"
            export CARGO_TARGET_DIR="$(pwd)/.local/cargo"
            export DATABASE_URL=sqlite://./.local/orion.sqlite
            mkdir -p "$CARGO_HOME"

            export PATH="$CARGO_HOME/bin:$NPM_CONFIG_PREFIX/bin:$PATH"

            figlet -f slant "Spatch!" -t | lolcat
          '';
        };
      }
    );
}
