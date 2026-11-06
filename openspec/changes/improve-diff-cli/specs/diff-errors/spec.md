## ADDED Requirements

### Requirement: Diff errors are reported in full
When a diff fails, the error text SHALL list every error, one per line, each prefixed with `left:` or
`right:` according to the input it was found in. The text SHALL be human-readable and SHALL NOT be a
debug dump of internal types.

#### Scenario: Multiple errors on both sides
- **WHEN** a diff fails with two errors in the left input and one in the right input
- **THEN** the message contains three lines: two beginning with `left:` and one beginning with `right:`, each describing the offending path and reason (e.g. `left: Item /notifications/0 is missing index key 'id'`)

#### Scenario: CLI prints readable errors
- **WHEN** `spatch diff` fails with diff errors
- **THEN** stderr shows the one-per-line form, and the exit code is non-zero
