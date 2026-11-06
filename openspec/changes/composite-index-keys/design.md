# Design

## Schema shape
`"x-spatch-indexKey": "id"` and `"x-spatch-indexKey": ["region", "code"]` are both valid. An empty
array, or an array containing non-strings, is an invalid declaration. It must not be silently
ignored (today `as_str()` returns `None` for non-strings, which falls back to index-based diffing
without notice); behaviour for invalid declarations is decided during implementation and covered by
a test.

## Engine
- `index_key()` returns an ordered list of field names (a single-element list for the string form).
- `build_key_map` builds a composite identity from every field's filter value (using the existing
  string/number/boolean conversion); an item missing any field, or with an unrepresentable value in
  any field, is an error for that field.
- Duplicate detection is on the full combination.
- A `push_filter` variant accepts multiple pairs. Pair order follows the schema's order, so output
  is deterministic.
- Removed/modified paths use the composite filter segment; `add` still appends via `/-`.

## Compatibility
Single-string schemas produce byte-identical patches. Error variants gain the field name(s); their
`Display` text for the single-key case stays unchanged.

## Key value escaping
Filter values containing `,` or `=` must round-trip through the path parser. Verify against
`src/path/parser.rs` while implementing; if unsupported, reject or escape such values and add a test.
