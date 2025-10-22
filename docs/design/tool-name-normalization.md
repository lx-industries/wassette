# Tool Name Normalization Analysis

## Overview

This document explains the tool name normalization strategy in Wassette and addresses concerns about potential name collisions when converting WebAssembly Component function names to MCP (Model Context Protocol) tool names.

## Normalization Strategy

The `normalize_name_component` function converts Component Model interface names to MCP-compliant tool names by:

1. Converting to lowercase
2. Replacing special characters (`:`, `/`, `.`) with underscores (`_`)
3. Preserving alphanumeric characters and hyphens (`-`)
4. Replacing other invalid characters with underscores

```rust
fn normalize_name_component(name: &str) -> String {
    name.to_lowercase()
        .chars()
        .map(|c| match c {
            ':' | '/' | '.' => '_',
            c if c.is_ascii_alphanumeric() || c == '-' => c,
            _ => '_',
        })
        .collect()
}
```

## WIT Specification Constraints

Per the [WebAssembly Component Model WIT specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md):

### Valid Name Formats

1. **Package names**: `namespace:package` (e.g., `wasi:http`, `local:demo`)
   - Uses `:` to separate namespace from package
   - Both parts use kebab-case: `[a-z][0-9a-z-]*`

2. **Interface names**: kebab-case labels (e.g., `types`, `my-interface`)
   - No special characters except hyphens
   - Format: `[a-z][0-9a-z-]*` with `-` separators

3. **Fully qualified names**: `namespace:package/interface` (e.g., `wasi:http/types`)
   - Uses `:` for namespace/package separation
   - Uses `/` to separate package from interface

### Characters in Valid WIT Names

- **Lowercase letters**: `[a-z]`
- **Digits**: `[0-9]`  
- **Hyphens**: `-` (preserved in normalization)
- **Colon**: `:` (only between namespace and package)
- **Slash**: `/` (only between package and interface)
- **NO underscores** in valid WIT identifiers
- **NO dots** in valid WIT identifiers (except in versioning)

## Collision Analysis

### Theoretical Collision Scenario

A collision could theoretically occur if:
- `wasi:http` → `wasi_http`
- `wasi_http` → `wasi_http` (if this were a valid name)

However, **`wasi_http` is NOT a valid WIT package name** per the specification.

### Valid WIT Names Cannot Collide

Testing shows that two different **valid** WIT names cannot collide:

| Valid Name 1 | Valid Name 2 | Normalized 1 | Normalized 2 | Collision? |
|--------------|--------------|--------------|--------------|------------|
| `foo:bar/baz` | `foo-bar:baz` | `foo_bar_baz` | `foo-bar_baz` | ❌ No |
| `wasi:io/streams` | `wasi-io:streams` | `wasi_io_streams` | `wasi-io_streams` | ❌ No |
| `pkg:test/my-interface` | `pkg:test/myinterface` | `pkg_test_my-interface` | `pkg_test_myinterface` | ❌ No |

The key insight is that **hyphens are preserved** in the normalization, which means different kebab-case structures produce different normalized names.

### Why Collisions Don't Occur in Practice

1. **Wasmtime validates components**: The wasmtime engine only accepts valid Component Model binaries that follow the WIT specification
2. **Invalid names are rejected**: Component names with underscores in package/interface names would be rejected during component creation
3. **Hyphens are preserved**: The main differentiator between valid WIT names (hyphens) is preserved during normalization

## Test Coverage

The test `test_no_collision_with_valid_wit_names` in `crates/component2json/src/lib.rs` demonstrates that valid WIT component names do not collide after normalization.

## Conclusion

**The original issue (#57) described a theoretical collision problem that does not occur in practice** because:

1. The Component Model specification constrains what characters can appear in valid names
2. Invalid names (with underscores or dots in inappropriate places) are rejected by wasmtime
3. The normalization preserves hyphens, which are the main differentiator in valid WIT names

No changes to the normalization strategy are needed. The current implementation correctly handles all valid WIT component names without collisions.

## References

- [WIT Specification](https://github.com/WebAssembly/component-model/blob/main/design/mvp/WIT.md)
- [Component Model Canonical ABI](https://github.com/WebAssembly/component-model/blob/main/design/mvp/CanonicalABI.md)
- [MCP Tool Name Specification](https://spec.modelcontextprotocol.io/specification/2024-11-05/server/tools/#tool-definition): `^[a-zA-Z0-9_-]{1,128}$`
