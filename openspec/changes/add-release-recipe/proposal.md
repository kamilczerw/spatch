# Proposal

## Why

Releases are cut by hand: tags exist locally (`v0.1.0`..`v0.6.0`) but there are no GitHub
Releases, and nothing guarantees a tag matches the version in `Cargo.toml`. Consumers have to pin
a commit SHA instead of a released version.

## What Changes

- Add a `release` recipe to the `Justfile` that reads the version from `Cargo.toml`, verifies the
  working tree is clean, creates the tag `v<version>`, pushes it, and creates a GitHub Release with
  generated notes (`gh release create v<version> --generate-notes`).
- `Cargo.toml` is the single source of truth for the version; the tag is always derived from it.
  (The flake already follows `Cargo.toml`, see `fix-nix-flake`.)

## Capabilities

### New Capabilities
- `release-process`: how a version is tagged and published.

### Modified Capabilities

## Impact

- `Justfile` only. Requires `git` and an authenticated `gh` CLI at release time.
