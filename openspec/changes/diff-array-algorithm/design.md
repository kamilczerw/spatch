# Design

## Context

`diff_array_indexed` has four hard-coded fast paths plus a positional fallback. It is one long
function with repeated path-building and patch-folding. `DiffOptions` is `Copy` and
`#[non_exhaustive]`, with builder methods (`with_granularity`, ...); the CLI mirrors options with
a `ValueEnum` plus `From` (see `Granularity` in `src/cli/mod.rs`).

## Decisions

- **Extraction before addition.** Phase 1 changes no behavior. Extract `item_path`, `remove_at`,
  `add_at`; move the existing fast paths and fallback, unchanged, into
  `diff_array_indexed_legacy`. `diff_array_indexed` becomes a dispatcher on the new option.
- **Library, not hand-written.** Use `similar` (Apache-2.0) for Myers. Do not write our own.
  OmniDiff (`github.com/ivankovic/omnidiff`) was read only to confirm the approach (Myers LCS on
  element sequences, then resolving the leftovers). It is AGPL-3.0-only: **no code may be copied
  or ported from it.**
- **Verified `similar` API (2.7.0, read from source).**
  - Use the low-level `similar::algorithms::myers::diff(&mut hook, old, 0..n, new, 0..m)` with
    `old`/`new` as `&[Value]`. Bounds are `Index<usize>` plus `PartialEq`, which `[Value]` meets;
    no `Hash` or `Ord` needed, so `capture_diff_slices` (needs `Hash + Eq`) is not used.
  - Hook: `Replace<Capture>` (both public in `similar::algorithms`). `Replace` merges an adjacent
    delete + insert into one `DiffOp::Replace { old_index, old_len, new_index, new_len }`, so the
    ops are already hunks between `Equal` runs; no hand-written hunk grouping.
  - Depend with `default-features = false`: the `algorithms` module is not feature-gated, only
    the text API is. This keeps `unicode-segmentation` etc. out.
  - Greedy prefix/suffix: `conquer` strips the common prefix and suffix before searching, so
    `[0,360,360,600,...]` -> `[0,360,600,...]` deletes index 2, as the spec requires.
  - Cost ceiling: no edit-count cap exists, only `diff_deadline(..., Option<Instant>)`, a wall
    clock deadline after which it degrades to a valid but non-minimal delete + insert. We call
    plain `diff` (no deadline, deterministic output). Time O(ND), memory O(N).
    `ponytail: no deadline; switch to diff_deadline if huge arrays with many edits show up.`
  - MSRV: similar declares rust-version 1.60, below the project's 1.89.
- **Pipeline.**
  1. `Replace<Capture>` yields `Equal`, `Delete`, `Insert`, `Replace` ops (hunks).
  2. For a hunk with `dl` deletes at old index `o` and `il` inserts at new index `n`, pair the first
     `k = min(dl, il)` positionally and recurse with `diff_recursive` on `left[o+i]`,
     `right[n+i]` (keeps granular output such as `replace /0/duration`).
  3. Leftover deletes (`dl > k`) become `remove` at `o+k .. o+dl`; leftover inserts (`il > k`)
     become `add` at `o+k`.
  4. Emit hunks **right to left**. All indices then refer to the original left array, with no offset
     bookkeeping. Within a hunk, emit leftovers (higher indices) before the paired recursions.
- **Descending removals.** Right-to-left emission and descending order inside a hunk guarantee it.
  RFC 6902 (section 4) applies ops sequentially and `remove` shifts later elements down, so
  descending keeps every index stable. Repeating `remove /p` would also be valid, but one rule is
  simpler.
- **Insert ordering.** Several inserts at the same index are emitted in reverse so the final order
  is right (matches legacy add-to-front). Inserts at the end (`index == len(left)`) use `-` in
  forward order (matches legacy append).
- **Runtime selection.** New `DiffOptions` setting, e.g. `ArrayAlgorithm::{Positional, Myers}`
  with `with_array_algorithm(..)`, plus CLI `--array-algorithm <positional|myers>` mirroring
  `--granularity` (`ValueEnum` + `From`). `item_options` is passed by value and `DiffOptions` is
  `Copy`, so the setting reaches nested arrays for free.
- **Naming.** The public name is `Myers` on purpose: if we later drop the `similar` crate we can re-implement Myers internally without renaming the API. The legacy name stays `Positional`.
- **Legacy untouched.** `diff_array_indexed_legacy` is the comparison baseline. The earlier
  concern that descending order changes `diff_array_indexed_should_handle_pure_remove_from_front`
  (`remove /0, /0`) disappears: that test keeps passing under the default. The new path produces
  `remove /1, /0` there, which is valid and covered by its own test.
- **Compact needs no change.** `Compact` collapses to a whole-value `replace` only when a
  `Granular` diff touches a majority of fields. A minimal script is already small.

## Risks / Trade-offs

- Duplicate values: any LCS is valid, and the greedy prefix/suffix makes the common cases
  deterministic. A different but equivalent patch is possible for ambiguous inputs.
- Moves are reported as remove + add (no move detection).
- `similar` becomes a runtime dependency even when the flag is unused (runtime option chosen over a
  Cargo feature so both algorithms can be compared in one run and one test).

## Decisions

- **Default algorithm: `Positional`, for now (user decision).** Flipping the default to `Myers`
  (and then deleting the legacy path) is a separate future change, after comparing outputs.
- **Names:** `ArrayAlgorithm::{Positional, Myers}` and `--array-algorithm <positional|myers>`
  (user decision).
