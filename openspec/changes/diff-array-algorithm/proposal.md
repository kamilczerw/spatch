# Proposal

## Why

When elements are added to or removed from an unkeyed array, `diff_array_indexed`
(`src/diff/engine.rs`) only recognizes four hard-coded cases (truncate, remove-from-front, append,
add-to-front). Anything else falls back to an index-by-index diff, so every element after the edit
looks changed and the patch hides the real edit.

- `[0,360,360,600,1800,3600]` -> `[0,360,600,1800,3600]` emits three `replace` ops plus
  `remove /5`, where a single `remove /2` is enough.
- `[0,360,600,1800,3600]` -> `[0,360,360,600,1800,1800,3600]` (two separate inserts) cannot be
  expressed by any single prefix/suffix rule; the minimal patch is `add /2` and `add /3`.

## What Changes

- Add a second array-diff algorithm that computes the minimal edit script with Myers LCS from the
  `similar` crate (new dependency, Apache-2.0), then turns equal/delete/insert hunks into
  `remove`/`add` ops and recurses into paired leftovers to keep granular `replace` output.
- Emit hunks right to left so every index refers to the original left array and removals come
  out in descending index order (valid under RFC 6902, where ops apply sequentially).
- The new algorithm is **selectable at runtime** (`DiffOptions` setting + matching CLI flag) so its
  output can be compared with the current one. The current algorithm stays the default and is left
  unchanged as the comparison baseline.
- Refactor first: extract small helpers from `diff_array_indexed` and move the existing code into
  `diff_array_indexed_legacy` before adding anything.

## Capabilities

### New Capabilities
- `array-diff`: how unkeyed arrays are diffed into minimal, RFC 6902-compliant patches.

### Modified Capabilities

## Impact

- `src/diff/engine.rs` (`diff_array_indexed` and tests), `src/diff/options.rs` (new setting),
  `src/cli/mod.rs` and `src/cli/diff.rs` (new flag), `Cargo.toml` (`similar`).
- Public API: one new `DiffOptions` setting (`DiffOptions` is `#[non_exhaustive]` and `Copy`, so
  this is additive). Default behavior unchanged, so no output changes unless the flag is set.
- Result is identical under `Granular` and `Compact` for the examples above: a minimal script
  has nothing for `Compact` to collapse.
- Out of scope: moved elements (a move is reported as remove + add); keyed arrays
  (`x-spatch-indexKey`), which use a different path.
