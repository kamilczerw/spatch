## ADDED Requirements

### Requirement: Flake evaluates and builds
The flake SHALL evaluate without errors and SHALL build the default package using the locked
`Cargo.lock` dependencies, without any placeholder or fake hash.

#### Scenario: Default package builds
- **WHEN** a user runs `nix build` on the flake
- **THEN** evaluation succeeds and the `spatch` package is built

#### Scenario: Dev shell evaluates
- **WHEN** a user runs `nix develop` on the flake
- **THEN** evaluation succeeds and the shell provides `rustc` and `cargo`

### Requirement: Package version follows Cargo.toml
The flake's package version SHALL be read from the `version` field of `Cargo.toml`, so that the
two cannot drift apart.

#### Scenario: Version matches the crate
- **WHEN** `Cargo.toml` declares version `X.Y.Z`
- **THEN** the flake's default package reports version `X.Y.Z`

#### Scenario: Version bump needs no flake edit
- **WHEN** the version in `Cargo.toml` is changed
- **THEN** the flake reports the new version without any change to `flake.nix`
