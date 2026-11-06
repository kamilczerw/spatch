
# Default: show help
default:
    @just --list

# Run all tests using cargo nextest
test test="":
    cargo nextest run {{test}}

# Run tests with cargo nextest and watch for changes
test-watch test="":
  cargo watch -x "nextest run {{test}}"

coverage:
  cargo llvm-cov nextest --open

# Tag v<Cargo.toml version>, push it and create a GitHub Release (pass any `dry` value to only echo the git/gh commands)
release dry="":
    #!/usr/bin/env bash
    set -euo pipefail
    version=$(sed -n 's/^version *= *"\(.*\)"/\1/p' Cargo.toml | head -1)
    [ -n "$version" ] || { echo "cannot read version from Cargo.toml" >&2; exit 1; }
    [ -z "$(git status --porcelain)" ] || { echo "working tree is dirty" >&2; exit 1; }
    ! git rev-parse -q --verify "refs/tags/v$version" >/dev/null || { echo "tag v$version already exists" >&2; exit 1; }
    run=""; [ -z "{{dry}}" ] || run=echo
    $run git tag "v$version"
    $run git push origin "v$version"
    $run gh release create "v$version" --generate-notes

