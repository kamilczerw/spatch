# Tasks

## 1. Tests first

- [ ] 1.1 Unit test: `DiffErrorSummary` `Display` lists all errors, one per line, with `left:`/`right:` prefixes (left only, right only, both)
- [ ] 1.2 CLI-level test: `--granularity granular` yields nested ops where default yields a whole-object `replace`
- [ ] 1.3 CLI-level test: invalid `--granularity` value is rejected
- [ ] 1.4 CLI-level test: failing diff prints the readable form and exits non-zero

## 2. Implement

- [ ] 2.1 Rewrite `Display for DiffErrorSummary` (`src/diff/error.rs`); keep the empty-summary fallback sensible
- [ ] 2.2 Add `--granularity` (clap `ValueEnum`, default `compact`) to `DiffArgs` and wire to `DiffOptions::with_granularity`
- [ ] 2.3 Make errors print via `Display` (e.g. handle the error in `main` instead of returning it) with exit code 1

## 3. Verify

- [ ] 3.1 `just test` passes
- [ ] 3.2 Update README / CLI help text for the new option
