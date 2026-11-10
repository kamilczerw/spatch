# Tasks

## 1. Baseline

- [x] 1.1 Run `just test`; confirm the only failure is `diff_array_indexed_should_handle_pure_remove_from_middle`

## 2. Extract (no behavior change)

- [x] 2.1 Extract `item_path`, `remove_at`, `add_at` helpers in `src/diff/engine.rs`
- [x] 2.2 Move the existing fast paths and fallback, unchanged, into `diff_array_indexed_legacy`; keep `diff_array_indexed` as a thin wrapper
- [x] 2.3 Add docstrings; run `just test` and confirm the same single failure as 1.1

## 3. Option and dependency (tests first)

- [x] 3.1 Test: `DiffOptions` defaults to the legacy algorithm and `with_array_algorithm(..)` selects the new one
- [x] 3.2 Add the setting to `src/diff/options.rs` (enum, builder, docstrings)
- [x] 3.3 Add `similar = { version = "2.7", default-features = false }` to `Cargo.toml`
- [x] 3.4 Add `--array-algorithm` to `DiffArgs` in `src/cli/mod.rs` (`ValueEnum` + `From`, like `Granularity`) and pass it in `src/cli/diff.rs`; add a CLI test in `tests/cli.rs`
- [x] 3.5 Make `diff_array_indexed` dispatch on the setting; the new branch is a stub until phase 4

## 4. Myers algorithm (tests first)

- [x] 4.1 Tests for: example (a) single `remove /thresholds/2`; example (b) `add /2`, `add /3`; duplicates; pure append, prepend, truncate (same output as legacy); remove from front (`/1`, `/0`); changed element between equals (`replace` on the field); every case applies to `right` via `patch::apply`
- [x] 4.2 Test: both algorithms, applied to the same inputs, yield a document equal to `right`
- [x] 4.3 Add `diff_ops(left, right) -> Vec<DiffOp>` using `myers::diff` with `Replace<Capture>`
- [x] 4.4 Add `hunk_patch(hunk, ...)` (pair, recurse, leftover removes/adds) as a small function
- [x] 4.5 Add `diff_array_indexed_myers` emitting hunks right to left; trailing inserts use `-`
- [x] 4.6 Update `diff_array_indexed_should_handle_pure_remove_from_middle` to select the new algorithm (it must pass there); the legacy path keeps its existing tests unchanged
- [x] 4.7 Docstrings; `just test`

## 5. Compare

- [ ] 5.1 Run `spatch diff` on real inputs with both values of `--array-algorithm`; note differences for the default-flip decision (partial: example (b) compared; real inputs still to do)
- [x] 5.2 `just test` passes; `openspec validate diff-array-algorithm` passes
