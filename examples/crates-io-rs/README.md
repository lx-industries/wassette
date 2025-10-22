# Crates.io Example (Rust)

This example demonstrates a Rust-based WebAssembly Component that uses HTTP capabilities to fetch crate information from crates.io. This example showcases how to build components that interact with external APIs while running in Wassette's secure sandbox.

For more information on installing Wassette, please see the [installation instructions](https://github.com/microsoft/wassette?tab=readme-ov-file#installation).

## Implementation

This example uses the [Spin SDK](https://github.com/fermyon/spin) to provide HTTP client capabilities and async runtime support. The Spin SDK offers a mature, production-ready approach to building WebAssembly components with HTTP networking.

## Building

This example uses the standard Rust build process with an additional documentation injection step:

```bash
# Build the component
cargo component build --release

# From repository root: inject WIT documentation into the component
just inject-docs examples/crates-io-rs/target/wasm32-wasip1/release/crates_io_rs.wasm examples/crates-io-rs/wit
```

The documentation injection embeds the WIT interface documentation into the WASM binary, making it available to AI agents when they discover this tool. See [`wit/world.wit`](wit/world.wit) for the documented interface.

For more information about documenting components, see the [Documenting WIT Interfaces](../../docs/cookbook/documenting-wit.md) guide.

## Usage

To use this component, load it and then request crate information.

**Load the component:**

```
Please load the crates.io component from the examples/crates-io-rs directory
```

**Get crate information:**

```
Please get information about the "serde" crate from crates.io
```

The component will fetch and return information including:
- Crate name and description
- Latest version
- Total and recent download counts
- List of recent versions

## Policy

By default, WebAssembly (Wasm) components do not have any access to the host machine or network. The `policy.yaml` file is used to explicitly define what network resources are made available to the component. This ensures that the component can only access the resources that are explicitly allowed.

Example:

```yaml
version: "1.0"
description: "Permission policy for crates-io-rs example in wassette"
permissions:
  network:
    allow:
      - host: "https://crates.io/"
```

This policy allows the component to make HTTP requests only to the crates.io API, following the principle of least privilege.

## Implementation Details

The component uses:
- **Spin SDK**: For async HTTP requests within the WebAssembly runtime
- **Serde**: For JSON serialization/deserialization
- **Crates.io API**: Public API for fetching crate metadata

The source code can be found in [`src/lib.rs`](src/lib.rs).

## About WASIp3

WASIp3 (WASI Preview 3) is an upcoming version of the WebAssembly System Interface that introduces native async support through the Component Model. As of Wasmtime v37, WASIp3 is available as an experimental preview with the `0.3.0-rc-2025-08-15` tag, but it is not yet production-ready.

The key innovation in WASIp3 is its native async primitives, which allow WebAssembly components to perform non-blocking I/O operations at the component model level. This differs from the approach used in this example, which relies on Spin SDK's async runtime.

When WASIp3 becomes production-ready in future Wasmtime releases, components can be migrated to use:
- `wasi:http/outgoing-handler@0.3.0` - For making async HTTP requests
- `wasi:cli/environment@0.3.0` - For accessing environment variables
- Native async primitives defined in the Component Model

To experiment with WASIp3 features in Wasmtime v37+, use: `-Sp3 -Wcomponent-model-async`

## Example Output

```markdown
# serde

**Description:** A generic serialization/deserialization framework

**Latest version:** 1.0.215

**Total downloads:** 423847291

**Recent downloads:** 28493821

**Recent versions:**

- 1.0.215
- 1.0.214
- 1.0.213
- 1.0.212
- 1.0.211
```
