# Tasks

Prerequisite: `improve-diff-cli` is implemented.

## 1. Tests first

- [ ] 1.1 Engine test: array with a missing key falls back to index paths; sibling keyed array keeps semantic paths
- [ ] 1.2 Engine test: each error kind (missing, non-string, duplicate, non-object) triggers fallback and yields a warning with path and side
- [ ] 1.3 Engine test: nested keyed array inside a fallback array stays keyed
- [ ] 1.4 Engine test: strict mode returns the errors as today
- [ ] 1.5 CLI test: lenient prints patch on stdout and warnings on stderr, exit 0; `--strict` prints nothing on stdout, exit non-zero

## 2. Implement

- [ ] 2.1 Decide the `diff()` API shape (see design.md) and update doc examples
- [ ] 2.2 Fall back inside `diff_array_keyed` per array and collect warnings
- [ ] 2.3 Add strict option to `DiffOptions` (default lenient) and `--strict` CLI flag
- [ ] 2.4 Print warnings to stderr in the CLI

## 3. Verify

- [ ] 3.1 `just test` passes; existing "should_fail" diff tests are updated to run in strict mode
- [ ] 3.2 Update README and CLI help
