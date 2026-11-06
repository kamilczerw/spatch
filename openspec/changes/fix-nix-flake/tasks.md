# Tasks

## 1. Verify the failure first

- [ ] 1.1 Run `nix flake check` (or `nix build`) and record the current eval/build errors

## 2. Fix the flake

- [ ] 2.1 Delete the `cargoSha256 = lib.fakeSha256;` line
- [ ] 2.2 Delete the `++ lib.optionals;` line from the dev shell packages
- [ ] 2.3 Replace the hardcoded `version` with `(lib.importTOML ./Cargo.toml).package.version`

## 3. Verify

- [ ] 3.1 `nix flake check` passes
- [ ] 3.2 `nix build` succeeds and the result reports the `Cargo.toml` version
- [ ] 3.3 `nix develop -c cargo --version` works
