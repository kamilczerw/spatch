## ADDED Requirements

### Requirement: Selectable array diff algorithm
The system SHALL let the caller choose, at runtime, between the existing positional array
algorithm and the Myers algorithm, through a `DiffOptions` setting and a matching CLI flag.
When the setting is not given, the existing positional algorithm SHALL be used and its output
SHALL be unchanged.

#### Scenario: Default is the existing algorithm
- **WHEN** an array is diffed without selecting an algorithm
- **THEN** the patch is identical to the output before this change

#### Scenario: CLI flag selects the new algorithm
- **WHEN** `spatch diff` runs with `--array-algorithm myers`
- **THEN** unkeyed arrays are diffed with the Myers algorithm

### Requirement: Myers diff yields a minimal patch
Under the Myers algorithm, when `right` equals `left` with any number of elements removed
and/or inserted anywhere, the diff SHALL consist only of `remove` and `add` operations for those
elements. Unchanged elements SHALL NOT produce operations. An element that changed in place SHALL
be diffed recursively, keeping the nested `replace` output.

#### Scenario: Remove from the middle
- **WHEN** `[{"duration":0},{"duration":360},{"duration":360},{"duration":600},{"duration":1800},{"duration":3600}]` is diffed against the same array without index 2
- **THEN** the patch is exactly `remove /thresholds/2`

#### Scenario: Two separate inserts
- **WHEN** `[0,360,600,1800,3600]` is diffed against `[0,360,360,600,1800,1800,3600]` (as `{"duration":n}` objects under `thresholds`)
- **THEN** the patch is `add /thresholds/3` (value `1800`) followed by `add /thresholds/2` (value `360`), or an equivalent two-operation patch
- **AND** applying it to the left array yields the right array

#### Scenario: Changed element between unchanged ones
- **WHEN** `[{"d":1},{"d":2},{"d":3}]` is diffed against `[{"d":1},{"d":9},{"d":3}]`
- **THEN** the patch is `replace /1/d` with `9`

#### Scenario: Same result in Compact and Granular
- **WHEN** the remove-from-the-middle arrays are diffed with `Compact` or `Granular` granularity
- **THEN** both produce the same single-`remove` patch

### Requirement: Removals are emitted in descending index order
Under the Myers algorithm, when a diff removes several array elements, the `remove`
operations SHALL be ordered by descending index, so each operation's index is unaffected by the
ones before it. The patch SHALL remain valid under RFC 6902, which applies operations
sequentially.

#### Scenario: Remove from the front
- **WHEN** `["a","b","c","d"]` is diffed against `["c","d"]`
- **THEN** the patch is `remove /1`, `remove /0`
- **AND** applying it to the left array yields `["c","d"]`

#### Scenario: Remove several from the middle
- **WHEN** `["a","b","c","d","e"]` is diffed against `["a","e"]`
- **THEN** the patch is `remove /3`, `remove /2`, `remove /1`

### Requirement: Appends use the end marker
Under the Myers algorithm, elements inserted after the last element of `left` SHALL be
emitted as `add` operations on `-` in order of appearance.

#### Scenario: Pure append
- **WHEN** `["a","b"]` is diffed against `["a","b","c","d"]`
- **THEN** the patch is `add /-` with `"c"`, then `add /-` with `"d"`
