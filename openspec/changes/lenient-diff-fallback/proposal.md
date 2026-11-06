# Proposal

## Why

Today a single keyed array whose items cannot be keyed (e.g. an item missing the field named by
`x-spatch-indexKey`) makes the whole `diff()` return `Err`, and the CLI exits 1 with no patch, even
though every other part of the document diffed fine. Schemas are sometimes wrong or incomplete; the
useful answer is "this array fell back to positions, here is why", not "no output".

## What Changes

- **Lenient by default:** when a keyed array has items that cannot be keyed, diff that array only
  with index-based diffing, emit the patch, and report the fallback as a warning. Other arrays keep
  their semantic paths.
- The library `diff()` returns warnings alongside the patch (API change, details in design.md).
- The CLI prints warnings to stderr (the patch stays on stdout) and exits 0.
- `--strict` restores the current behaviour: any such error fails the diff.

## Capabilities

### New Capabilities
- `diff-fallback`: per-array fallback to index-based diffing, its warnings, and strict mode.

### Modified Capabilities

## Impact

- `src/diff/engine.rs` (`diff_array_keyed`), `src/diff/mod.rs` (`diff` signature/result),
  `src/diff/error.rs`, `src/cli/diff.rs`, `src/cli/mod.rs`.
- Depends on `improve-diff-cli` (shares the CLI args and the readable `left:`/`right:` formatting).
- **BREAKING (library):** the return type of `diff()` changes unless a new function is added.
