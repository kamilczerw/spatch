# Design

## Which errors trigger fallback
Current keyed-diff errors (`src/diff/error.rs`): `MissingIndexKey`, `NonStringIndexKey`,
`DuplicateIndexKey`, `NonObjectArrayItem`. All describe an array that cannot be keyed reliably.
Proposed: all four trigger fallback of that array (left or right array failing to key means the
whole array pair is diffed by index). To confirm during implementation.

## Mechanics
`diff_array_keyed` already builds key maps for both sides and collects errors. If either side
returns key-map errors, call `diff_array_indexed` for that array instead and turn the errors into
warnings, each retaining its path and `left`/`right` side. Errors from nested arrays are handled
at their own level, so a fallback never spreads to siblings or parents.

## API
`diff()` currently returns `Result<Patch, DiffErrorSummary>`. Options: (a) change to return a
struct with `patch` and `warnings`; (b) keep `diff()` strict and add a new function. Recommendation:
(a) with a `DiffOptions` strict flag (default lenient), because the default behaviour changes.
Decide during implementation; note it is a breaking change for library users.

## Open question
Should strictness depend on the schema? If the index key is listed in the item schema's
`required`, a missing key is invalid data and arguably should stay an error; if it is not
required, a missing key is valid data and fallback is the right answer. Out of scope for the first
version (lenient for everything unless `--strict`); decide whether to add it later.
