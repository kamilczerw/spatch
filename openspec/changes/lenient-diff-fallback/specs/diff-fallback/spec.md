## ADDED Requirements

### Requirement: Unkeyable arrays fall back to index-based diffing
When an array's schema declares `x-spatch-indexKey` but its items cannot be keyed (an item is missing the key, has an unrepresentable key value, duplicates another item's key, or is not an object), the diff SHALL diff that array using index-based paths and SHALL still produce a patch. The fallback SHALL apply to that array only; other arrays SHALL keep their schema-aware paths.

#### Scenario: One bad array, one good array
- **WHEN** `groups` is keyed by `id` and diffs normally, and an item in `notifications` (also keyed by `id`) has no `id`
- **THEN** the patch contains semantic paths for `groups` (e.g. `/groups/[id=b]`), index-based paths for `notifications`, and no error is returned

#### Scenario: Fallback does not spread to nested arrays
- **WHEN** an outer array falls back to index-based diffing and its items contain an inner array that is correctly keyed
- **THEN** the inner array is diffed with its own semantic paths

### Requirement: Fallbacks are reported as warnings
Every fallback SHALL be reported as a warning identifying the array path, the reason, and whether it concerned the left or right input. The library SHALL return warnings alongside the patch, and the CLI SHALL print them to stderr while writing only the patch to stdout.

#### Scenario: CLI reports a fallback
- **WHEN** `spatch diff` falls back for `/notifications`
- **THEN** stderr contains a warning naming `/notifications` and the reason (e.g. `Item /notifications/1 is missing index key 'id'`), stdout contains only the JSON patch, and the exit code is 0

#### Scenario: No fallback, no warnings
- **WHEN** every keyed array can be keyed
- **THEN** no warnings are produced

### Requirement: Strict mode keeps failing on unkeyable arrays
`spatch diff --strict` SHALL fail the diff, produce no patch and exit non-zero when any array would have fallen back, reporting the errors in the same form as `diff-errors`.

#### Scenario: Strict failure
- **WHEN** the user runs `spatch diff a.json b.json --schema s.json --strict` and an item lacks its index key
- **THEN** no patch is printed, the errors are listed one per line on stderr, and the exit code is non-zero
