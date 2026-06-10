# Spatch - JSON Patch with Schema‑Aware Array Paths

A Rust library and CLI for working with JSON Patch (RFC 6902) that adds optional,
schema‑aware paths for stable array element addressing, while always producing
and consuming standard JSON Patch operations.

This tool solves a common problem with JSON Patch: array elements are addressed by
index, which makes diffs fragile and patches noisy when arrays are reordered or elements
are inserted/removed. When a JSON Schema is available, this crate allows you to address
array elements by semantic identity (e.g. a key field), and compiles those paths
down to ordinary RFC 6902 patches.

📣 **Design discussion & feedback wanted:**\
[https://github.com/kamilczerw/spatch/discussions/1](https://github.com/kamilczerw/spatch/discussions/1)

## Key Ideas

- **RFC 6902 remains the wire format** -
  Generated patches are always valid JSON Patch. No extensions, no custom ops.

- **Semantic paths are optional and schema-enhanced**
  Semantic array paths can be resolved without a schema (as long as the JSON contains
  the referenced key/value), but a JSON Schema is used to generate schema-aware diffs
  (and to disambiguate/validate identity rules when needed). Without a schema, diff
  output is standard index-based JSON Patch.

- **Array elements are addressed by identity, not position**
  Example semantic path:

  ```
  /arr/[id=foo]/bar
  ```

  Given the JSON:

  ```json
  {
    "arr": [{ "id": "foo", "bar": "baz" }]
  }
  ```

## Features

- Generate JSON Patch diffs
  - Pure RFC 6902 (no schema)
  - Schema‑aware diffs using semantic array paths
  - Compact or granular object diffs, depending on whether you want smaller
    patches or review-friendly patches
- Apply JSON Patch operations from a file
- Read values at a path
  - Standard JSON Pointer
  - Schema‑aware semantic paths
- Usable as both a **library** and a **CLI**

## Usage

There are 2 ways to use this crate - as cli or as a library.

### CLI

#### Query

The query language is a standard JSON Pointer with added support for resolving array
elements by their identity properties. For example, given the following JSON:

```json
{
  "list": [
    { "id": "item-1", "name": "Item 1", "value": 10 },
    { "id": "item-2", "name": "Item 2", "value": 20 }
  ]
}
```

You can query by json pointer:

```bash
cat examples/simple.json | spatch query '/list/0'
```

Or by semantic path:

```bash
cat examples/simple.json | spatch query '/list/[id=item-1]'
```

The 2 above commands will output the same result:

```json
{ "id": "item-1", "name": "Item 1", "value": 10 }
```

You can also read the leaf value directly:

```bash
cat examples/simple.json | spatch query '/list/[id=item-1]/value'
```

#### Diff

The `diff` command generates a JSON Patch between 2 JSON documents.
It operates in 2 modes - pure RFC 6902 mode (index-based array addressing),
or schema-aware mode (semantic array addressing).

By default, `spatch diff` operates in pure RFC 6902 mode:

```bash
spatch diff examples/simple.json examples/simple-new.json
```

Will output a standard JSON Patch with index-based array paths.

```json
[
  {
    "op": "replace",
    "path": "/list/1",
    "value": {
      "id": "item-2",
      "name": "Item Two",
      "value": 200
    }
  }
]
```

To use the schema-aware mode, provide a JSON Schema with identity definitions

```bash
spatch diff --schema examples/simple.schema.json examples/simple.json examples/simple-new.json
```

Will produce a JSON Patch with semantic array paths:

```json
[
  {
    "op": "replace",
    "path": "/list/[id=item-2]",
    "value": {
      "id": "item-2",
      "name": "Item Two",
      "value": 200
    }
  }
]
```

> [!IMPORTANT]
>
> To let spatch know which property to use as identity key for array elements, you
> **MUST** provide a JSON Schema that defines the array with `x-spatch-indexKey: "{identity-property-name}"`.
> Otherwise, spatch will fall back to index-based addressing.

Schema-aware diffing also follows local JSON Schema `$ref`s while walking
`properties` and `items`. This means each nested array can define its own
`x-spatch-indexKey`, even when item schemas are shared through `$defs`:

```json
{
  "properties": {
    "tracks": {
      "type": "array",
      "x-spatch-indexKey": "id",
      "items": { "$ref": "#/$defs/track" }
    }
  },
  "$defs": {
    "track": {
      "type": "object",
      "properties": {
        "levels": {
          "type": "array",
          "x-spatch-indexKey": "id",
          "items": { "$ref": "#/$defs/level" }
        }
      }
    },
    "level": {
      "type": "object",
      "properties": {
        "xp": {}
      }
    }
  }
}
```

For data like `{ "tracks": [{ "id": "free", "levels": [{ "id": 1, "xp": 100 }] }] }`, numeric identity
values are emitted directly in semantic paths, for example:

    /tracks/[id=free]/levels/[id=1]/xp

`x-spatch-indexKey` values may be strings, numbers, or booleans, producing filters such as
`[id=item-2]`, `[id=1]`, or `[enabled=true]`. Object, array, and `null` identity
values are rejected because they cannot be represented safely in a semantic path.

### Library

Spatch is designed to be pleasant to use directly from Rust. The `diff` API takes
`DiffOptions`, so you can choose the patch shape that fits your product:

- use **compact** diffs when patches are stored, sent over the wire, or optimized
  for size;
- use **granular** diffs when patches will be reviewed by humans, shown in a UI,
  or used as audit-log entries;
- add a schema when arrays have stable identities and you want paths that survive
  inserts, removals, and reordering.

#### Stable array diffs with schema-aware paths

With a schema, array elements can be addressed by identity instead of by index.
That means the patch below points at `u-2` even though the array order changed.

```rust
use serde_json::json;
use spatch::diff::{diff, DiffOptions};

let schema = json!({
    "properties": {
        "users": {
            "x-spatch-indexKey": "id",
            "items": {
                "properties": {
                    "name": {}
                }
            }
        }
    }
});

let before = json!({
    "users": [
        {"id": "u-1", "name": "Ada"},
        {"id": "u-2", "name": "Grace"}
    ]
});

let after = json!({
    "users": [
        {"id": "u-2", "name": "Grace Hopper"},
        {"id": "u-1", "name": "Ada"}
    ]
});

let patch = diff(
    &before,
    &after,
    DiffOptions::new().with_schema(&schema).granular(),
)?;
```

The generated patch is stable and easy to understand:

```json
[
  {
    "op": "replace",
    "path": "/users/[id=u-2]/name",
    "value": "Grace Hopper"
  }
]
```

#### Nested `$ref` schemas and scalar identity values

Schema-aware diffs resolve local JSON Schema references such as
`{ "$ref": "#/$defs/track" }` while traversing schemas. This allows semantic
paths to continue through nested arrays:

```rust
use serde_json::json;
use spatch::diff::{diff, DiffOptions};

let schema = json!({
    "properties": {
        "tracks": {
            "x-spatch-indexKey": "id",
            "items": { "$ref": "#/$defs/track" }
        }
    },
    "$defs": {
        "track": {
            "properties": {
                "levels": {
                    "x-spatch-indexKey": "id",
                    "items": { "$ref": "#/$defs/level" }
                }
            }
        },
        "level": {
            "properties": {
                "rewards": {
                    "x-spatch-indexKey": "id",
                    "items": { "$ref": "#/$defs/reward" }
                }
            }
        },
        "reward": { "properties": { "amount": {} } }
    }
});

let before = json!({"tracks": [{"id": "free", "levels": [{
    "id": 1,
    "xp": 100,
    "rewards": [{"id": "reward-1", "amount": 100}]
}]}]});

let after = json!({"tracks": [{"id": "free", "levels": [{
    "id": 1,
    "xp": 150,
    "rewards": [{"id": "reward-1", "amount": 250}]
}]}]});

let patch = diff(
    &before,
    &after,
    DiffOptions::new().with_schema(&schema).granular(),
)?;
```

Example paths from that patch include a numeric level identity and a nested reward
identity:

```text
/tracks/[id=free]/levels/[id=1]/xp
/tracks/[id=free]/levels/[id=1]/rewards/[id=reward-1]/amount
```

The value of the property named by `x-spatch-indexKey` may be a string, number, or boolean.
Object, array, and `null` values are rejected and reported as diff errors instead
of being encoded into semantic path filters.

#### Choose compact or granular object diffs

`DiffOptions::new()` defaults to compact mode. Compact mode keeps patches small
and may replace a parent object when that is shorter than many nested operations.
When schema-aware diffing produces semantic paths, compact mode keeps those
semantic operations instead of collapsing them away, so identity filters such as
`[id=item-2]` or `[id=1]` remain visible in the patch.

```rust
use serde_json::json;
use spatch::diff::{diff, DiffOptions};

let before = json!({
    "settings": {
        "theme": "light",
        "language": "en",
        "notifications": true
    }
});

let after = json!({
    "settings": {
        "theme": "dark",
        "language": "pl",
        "notifications": false
    }
});

let compact_patch = diff(&before, &after, DiffOptions::new().compact())?;
```

When you care about readability, choose granular mode. Spatch keeps walking into
objects and emits the specific fields that changed:

```rust
let granular_patch = diff(&before, &after, DiffOptions::new().granular())?;
```

Example granular output:

```json
[
  { "op": "replace", "path": "/settings/theme", "value": "dark" },
  { "op": "replace", "path": "/settings/language", "value": "pl" },
  { "op": "replace", "path": "/settings/notifications", "value": false }
]
```

Both modes still produce JSON Patch operations. You can pick the representation
that is best for your users without changing the patch format your system stores
or transmits.

## Why This Exists

JSON Patch is a solid standard, but **index‑based array addressing is brittle**:

- Reordering arrays produces large, noisy diffs
- Insertions shift indices and invalidate patches
- Logical identity is lost

This crate keeps JSON Patch unchanged, but uses JSON Schema to recover **semantic
identity** for array elements, producing patches that are:

- More stable
- Easier to review
- Safer to apply
