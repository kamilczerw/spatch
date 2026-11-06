## ADDED Requirements

### Requirement: Release tag is derived from Cargo.toml
The `just release` command SHALL take the release version from the `version` field of
`Cargo.toml` and SHALL create the tag `v<version>`; the user SHALL NOT supply the version by hand.

#### Scenario: Tag matches the crate version
- **WHEN** `Cargo.toml` declares version `0.7.0` and the user runs `just release`
- **THEN** the tag `v0.7.0` is created

### Requirement: Release requires a clean tree
The `just release` command SHALL refuse to run, creating no tag, when the working tree has
uncommitted changes.

#### Scenario: Dirty working tree
- **WHEN** the working tree has uncommitted changes and the user runs `just release`
- **THEN** the command exits non-zero with a message and no tag or release is created

### Requirement: Release does not overwrite existing versions
The `just release` command SHALL fail without changes when the tag `v<version>` already exists.

#### Scenario: Tag already exists
- **WHEN** the tag `v<version>` already exists and the user runs `just release`
- **THEN** the command exits non-zero and nothing is pushed or published

### Requirement: Release publishes a GitHub Release
After tagging, `just release` SHALL push the tag and SHALL create a GitHub Release for it with
generated release notes.

#### Scenario: Successful release
- **WHEN** the tree is clean, the tag does not exist, and `gh` is authenticated
- **THEN** the tag `v<version>` is pushed and a GitHub Release `v<version>` exists with generated notes
