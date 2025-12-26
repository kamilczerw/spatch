# Spatch - JSON Patch with Schema‑Aware Array Paths

A Rust library and CLI for working with JSON Patch (RFC 6902) that adds optional,
schema‑aware paths for stable array element addressing, while always producing
and consuming standard JSON Patch operations.

This tool solves a common problem with JSON Patch: array elements are addressed by
index, which makes diffs fragile and patches noisy when arrays are reordered or elements
are inserted/removed. When a JSON Schema is available, this crate allows you to address
array elements by semantic identity (e.g. a key field), and compiles those paths
down to ordinary RFC 6902 patches.

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

> [!IMPORTANT] > **Index key**
>
> To let spatch know which property to use as identity key for array elements, you
> **MUST** provide a JSON Schema that defines the array with `indexKey: "{identity-property-name}"`.
> Otherwise, spatch will fall back to index-based addressing.

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
