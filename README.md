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

> [!NOTE]
> TODO: Add usage instructions and examples

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
