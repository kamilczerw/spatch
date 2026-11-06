# Tasks

## 1. Tests first

- [ ] 1.1 Engine tests: composite add/remove/modify produce `/arr/[a=1,b=2]/...` paths; partial overlap is not a match; reorder yields empty patch
- [ ] 1.2 Engine tests: string form output unchanged (existing tests stay green)
- [ ] 1.3 Error tests: missing field, unrepresentable value, duplicate combination name the field(s)
- [ ] 1.4 Schema test: empty array and non-string entries are rejected
- [ ] 1.5 Round-trip test: generated composite paths parse and resolve/apply against the left document; check values containing `,` and `=`

## 2. Implement

- [ ] 2.1 Parse `x-spatch-indexKey` as string or array in `src/diff/schema.rs` / `options.rs`
- [ ] 2.2 Generalise `build_key_map` and `diff_array_keyed` to a list of fields
- [ ] 2.3 Add a multi-pair filter constructor next to `push_filter`
- [ ] 2.4 Extend error variants with the field name(s), keeping single-key `Display` text

## 3. Verify

- [ ] 3.1 `just test` passes
- [ ] 3.2 Document the array form in the README and `DiffOptions` docs
