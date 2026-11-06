## ADDED Requirements

### Requirement: Index key may be a single field or a list of fields
An array schema's `x-spatch-indexKey` SHALL accept either a string naming one field or a non-empty array of strings naming several fields that together identify an item.

#### Scenario: Single field unchanged
- **WHEN** `x-spatch-indexKey` is `"id"`
- **THEN** diffing produces the same patch as before this change (e.g. `/items/[id=a]/qty`)

#### Scenario: Composite key in paths
- **WHEN** `x-spatch-indexKey` is `["region", "code"]` and the item `{ "region": "eu", "code": "a1", "qty": 1 }` changes `qty` to 2
- **THEN** the patch contains a `replace` at `/items/[region=eu,code=a1]/qty`

### Requirement: Composite identity uses all fields
Items SHALL be matched between the two inputs only when every listed field has the same value. Two items sharing some but not all fields SHALL be treated as different items.

#### Scenario: Partial overlap is not a match
- **WHEN** the left array has `{ "region": "eu", "code": "a1" }` and the right array has `{ "region": "eu", "code": "b2" }`
- **THEN** the patch removes `/items/[region=eu,code=a1]` and adds the new item

#### Scenario: Reordering produces no operations
- **WHEN** the same items appear in a different order in both inputs
- **THEN** the patch is empty

### Requirement: Composite key errors name the fields
Missing fields, unrepresentable values and duplicate combinations SHALL be reported as errors that identify the item path and the field name(s) involved.

#### Scenario: One field missing
- **WHEN** an item lacks `code` for key `["region", "code"]`
- **THEN** the error identifies the item path and the missing key `code`

#### Scenario: Duplicate combination
- **WHEN** two items have the same `region` and `code` values
- **THEN** a duplicate-key error identifies the item path and the combination

### Requirement: Invalid index key declarations are rejected
An `x-spatch-indexKey` that is an empty array, or contains a non-string entry, SHALL NOT be silently ignored.

#### Scenario: Empty array
- **WHEN** `x-spatch-indexKey` is `[]`
- **THEN** diffing reports an error about the invalid declaration
