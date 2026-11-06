# Tasks

## 1. Decide how to verify

- [ ] 1.1 Dry-run strategy: document how to exercise the recipe without publishing (e.g. against a scratch remote or with `gh` and `git push` echoed)

## 2. Implement `just release`

- [ ] 2.1 Read the version from `Cargo.toml` (fail clearly if it cannot be read)
- [ ] 2.2 Fail if the working tree is dirty
- [ ] 2.3 Fail if tag `v<version>` already exists
- [ ] 2.4 Create the tag, push it, run `gh release create v<version> --generate-notes`
- [ ] 2.5 Add a one-line doc comment so `just --list` describes the recipe

## 3. Verify

- [ ] 3.1 Dirty tree and existing tag both abort with no side effects
- [ ] 3.2 Dry run prints the exact `git` and `gh` commands for the current version
