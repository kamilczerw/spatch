# Proposal

## Why

`x-spatch-indexKey` accepts a single field name, so arrays whose items are identified by two or
more fields together (e.g. `{ "region": "eu", "code": "a1" }`) cannot be addressed semantically.
The path layer already supports this: `Segment::Filter` is a `Vec<(String, String)>` and the parser
accepts `/foo/[id=123, type=active]/bar`; the resolver and `remove` already match multiple
conditions. Only the schema key and the diff engine are single-valued.

## What Changes

- `x-spatch-indexKey` accepts either a string (today's form) or a non-empty array of strings.
- The diff engine keys array items by the combination of all listed fields, and emits paths with one
  filter segment carrying every pair, e.g. `/items/[region=eu,code=a1]/qty`.
- Missing-key, non-representable and duplicate errors name the key field(s) involved.
- The string form behaves exactly as before.

## Capabilities

### New Capabilities
- `index-key-schema`: how `x-spatch-indexKey` declares array item identity, including composite keys.

### Modified Capabilities

## Impact

- `src/diff/schema.rs` (`index_key` reads only a string via `as_str()`; an array value is currently
  silently ignored, causing an index-based fallback), `src/diff/options.rs` (`index_key()`),
  `src/diff/engine.rs` (`diff_array`, `diff_array_keyed` at ~158, `build_key_map` at ~232 which
  builds a `HashMap<String, Value>` from one field), `src/path/mod.rs` (`push_filter` adds a single
  pair), `src/diff/error.rs` (error variants carry a single `index_key: String`).
- Independent of the other changes; error wording should match `improve-diff-cli`.
