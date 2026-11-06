## ADDED Requirements

### Requirement: Diff granularity is selectable
`spatch diff` SHALL accept `--granularity <compact|granular>` and SHALL produce its patch using the
chosen granularity. When the option is omitted the granularity SHALL be `compact`.

#### Scenario: Granular keeps nested operations
- **WHEN** the user runs `spatch diff a.json b.json --granularity granular` on inputs where an object in an array changes several fields and no schema is given
- **THEN** the patch contains per-field operations instead of a whole-object `replace`

#### Scenario: Default is compact
- **WHEN** the user runs `spatch diff a.json b.json` without `--granularity`
- **THEN** the output is identical to running with `--granularity compact`

#### Scenario: Invalid value
- **WHEN** the user passes `--granularity` with a value other than `compact` or `granular`
- **THEN** the command exits non-zero and reports the accepted values
