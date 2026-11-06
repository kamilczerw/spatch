# Proposal

## Why

`flake.nix` does not evaluate or build: the package sets `cargoSha256 = lib.fakeSha256` next to
`cargoLock.lockFile` (a fake hash makes the fixed-output derivation fail, and a lock file needs no
hash), and the dev shell concatenates `++ lib.optionals;` (a two-argument function with no
arguments, which errors at eval time). The package version is hardcoded to `0.1.0` while
`Cargo.toml` says `0.6.0`, so the store path and `nix flake show` report the wrong version.

## What Changes

- Remove the `cargoSha256` line from the package definition.
- Remove the `++ lib.optionals;` line from the dev shell package list.
- Use `pkgs.rustc` and `pkgs.cargo` in the dev shell (`pkgs.rustPlatform.rustc` does not exist and
  fails evaluation with `attribute 'rustc' missing`).
- Derive the package `version` from `Cargo.toml` via `(lib.importTOML ./Cargo.toml).package.version`
  instead of hardcoding it.

## Capabilities

### New Capabilities
- `nix-packaging`: the flake builds the package and dev shell, and reports the crate's version.

### Modified Capabilities

## Impact

- `flake.nix` only. No Rust code changes.
