# Proposal

## Why

Two CLI gaps make `spatch diff` hard to use:

1. The library exposes `DiffGranularity::{Compact, Granular}` (`DiffOptions::with_granularity`), but
   the CLI only exposes `--schema`. The default, `Compact`, collapses an object change into a
   whole-object `replace` when that serialises smaller, which is unreadable as a diff. Compact keeps
   nested operations only when the path already contains a semantic `[key=value]` segment, so it
   hurts exactly where there is no key to anchor on (e.g. no schema).
2. Diff failures print as a Rust `Debug` dump (`main` returns `Result<(), Box<dyn Error>>`, so the
   runtime uses `Debug`). `DiffErrorSummary`'s `Display` (`src/diff/error.rs`) prints only the first
   error and does not say whether it came from the left or right input, so switching the CLI to
   `Display` alone would hide most errors.

## What Changes

- Add `--granularity <compact|granular>` to `spatch diff`, default `compact`, wired to
  `DiffOptions::with_granularity`.
- Change `DiffErrorSummary`'s `Display` to list every error, one per line, prefixed with `left:` or
  `right:` depending on the input it came from.
- Make the CLI print that `Display` form for diff errors instead of the `Debug` form, with exit code
  1 unchanged.

## Capabilities

### New Capabilities
- `diff-cli`: options accepted by `spatch diff` that control diff output.
- `diff-errors`: how diff failures are reported to the user.

### Modified Capabilities

## Impact

- `src/cli/mod.rs` (new argument), `src/cli/diff.rs` (option wiring, error printing), `src/main.rs`
  (error reporting), `src/diff/error.rs` (`Display`).
- Error text on stderr changes; stdout and exit code do not. Default diff output is unchanged.
- `lenient-diff-fallback` builds on this change (adds `--strict` to the same arguments).
