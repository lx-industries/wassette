# Display all available recipes when running `just` without arguments
default:
    @just --list

# Clean component target directories to avoid permission issues
clean-test-components:
    rm -rf examples/fetch-rs/target/
    rm -rf examples/filesystem-rs/target/

# Pre-build test components to avoid building during test execution
build-test-components:
    just clean-test-components
    (cd examples/fetch-rs && cargo build --release --target wasm32-wasip2)
    (cd examples/filesystem-rs && cargo build --release --target wasm32-wasip2)

# Run all tests including workspace and documentation tests
test:
    just build-test-components
    cargo test --workspace -- --nocapture
    cargo test --doc --workspace -- --nocapture

# Build the wassette binary in debug or release mode (default: debug)
build mode="debug":
    mkdir -p bin
    cargo build --workspace {{ if mode == "release" { "--release" } else { "" } }}
    cp target/{{ mode }}/wassette bin/

# Install wassette binary locally to /usr/local/bin (macOS only, requires sudo)
install-local mode="debug":
    #!/usr/bin/env bash
    set -euo pipefail
    if [[ "$(uname -s)" != "Darwin" ]]; then
        echo "install-local currently supports macOS only" >&2
        exit 1
    fi
    src="target/{{ mode }}/wassette"
    dst="/usr/local/bin/wassette"
    if [[ ! -f "$src" ]]; then
        echo "missing binary: $src" >&2
        echo "run 'just build {{ mode }}' before installing" >&2
        exit 1
    fi
    mkdir -p "$(dirname "$dst")"
    cp "$src" "$dst"
    codesign --force --sign - "$dst"

# Build all example components and copy them to bin/ directory (mode applies to Rust examples only)
build-examples mode="debug":
    mkdir -p bin
    (cd examples/fetch-rs && just build {{ mode }})
    (cd examples/filesystem-rs && just build {{ mode }})
    (cd examples/get-weather-js && just build)
    (cd examples/time-server-js && just build)
    (cd examples/eval-py && just build)
    (cd examples/gomodule-go && just build)
    cp examples/fetch-rs/target/wasm32-wasip2/{{ mode }}/fetch_rs.wasm bin/fetch-rs.wasm
    cp examples/filesystem-rs/target/wasm32-wasip2/{{ mode }}/filesystem.wasm bin/filesystem.wasm
    cp examples/get-weather-js/weather.wasm bin/get-weather-js.wasm
    cp examples/time-server-js/time.wasm bin/time-server-js.wasm
    cp examples/eval-py/eval.wasm bin/eval-py.wasm
    cp examples/gomodule-go/gomodule.wasm bin/gomodule.wasm
    
# Clean all build artifacts and remove bin directory
clean:
    cargo clean
    rm -rf bin

# Convert a WebAssembly component to JSON format for inspection
component2json path="examples/fetch-rs/target/wasm32-wasip2/release/fetch_rs.wasm":
    cargo run --bin component2json -p component2json -- {{ path }}

# Check if locally installed wassette binary matches current git commit
check-local-version:
    #!/usr/bin/env bash
    set -euo pipefail
    
    # Get current git commit
    current_commit=$(git rev-parse HEAD)
    short_commit=$(git rev-parse --short HEAD)
    
    # Get version from wassette binary
    if ! command -v wassette &> /dev/null; then
        echo "❌ wassette binary not found in PATH"
        echo "   Run 'just build && just install-local' to install"
        exit 1
    fi
    
    version_output=$(wassette --version)
    if [[ "$version_output" != *"BuildInfo{"* ]]; then
        echo "❌ wassette --version output is missing BuildInfo details"
        echo "   The installed binary is too old for this check"
        echo "   Run 'just build && just install-local' to update"
        exit 1
    fi
    version_commit=$(echo "$version_output" | grep -o 'GitRevision:"[^"]*"' | cut -d'"' -f2)
    
    echo "Current commit:  $current_commit ($short_commit)"
    echo "Wassette commit: $version_commit"
    echo ""
    
    if [[ "$current_commit" == "$version_commit" ]]; then
        echo "✅ Versions match! Your wassette binary is up to date."
    else
        echo "❌ Version mismatch!"
        echo "   Your wassette binary was built from a different commit."
        echo "   Run 'just build && just install-local' to update."
        
        # Show if current commit is ahead/behind
        if git merge-base --is-ancestor "$version_commit" "$current_commit" 2>/dev/null; then
            commits_ahead=$(git rev-list --count "$version_commit".."$current_commit")
            echo "   Current commit is $commits_ahead commits ahead of binary."
        elif git merge-base --is-ancestor "$current_commit" "$version_commit" 2>/dev/null; then
            commits_behind=$(git rev-list --count "$current_commit".."$version_commit")
            echo "   Current commit is $commits_behind commits behind binary."
        else
            echo "   Commits have diverged."
        fi
    fi

# Run wassette server with SSE transport and info logging
run RUST_LOG='info':
    RUST_LOG={{RUST_LOG}} cargo run --bin wassette serve --sse

# Run wassette server with streamable HTTP transport and info logging
run-streamable RUST_LOG='info':
    RUST_LOG={{RUST_LOG}} cargo run --bin wassette serve --streamable-http

# Run wassette server with filesystem example component loaded
run-filesystem RUST_LOG='info':
    RUST_LOG={{RUST_LOG}} cargo run --bin wassette serve --sse --plugin-dir ./examples/filesystem-rs

# Run wassette server with get-weather example component loaded (requires OPENWEATHER_API_KEY)
run-get-weather RUST_LOG='info':
    RUST_LOG={{RUST_LOG}} cargo run --bin wassette serve --sse --plugin-dir ./examples/get-weather-js

# Run wassette server with fetch-rs example component loaded
run-fetch-rs RUST_LOG='info':
    RUST_LOG={{RUST_LOG}} cargo run --bin wassette serve --sse --plugin-dir ./examples/fetch-rs

# Documentation commands
# Build the documentation book using mdbook
docs-build:
    cd docs && mdbook build

# Serve the documentation book and open in browser
docs-serve:
    cd docs && mdbook serve --open

# Serve the documentation book with auto-reload on changes
docs-watch:
    cd docs && mdbook serve

# CI Docker commands - automatically handle user mapping to prevent permission issues
# Build and run CI tests locally using Docker
ci-local:
    docker build \
        --build-arg USER_ID=$(id -u) \
        --build-arg GROUP_ID=$(id -g) \
        -f Dockerfile.ci \
        --target ci-test \
        -t wassette-ci-local .
    docker run --rm \
        -v $(PWD):/workspace \
        -w /workspace \
        -e GITHUB_TOKEN \
        wassette-ci-local just ci-build-test

# Build and test components for CI (standard tests)
ci-build-test:
    just build-test-components
    cargo build --workspace
    cargo test --workspace -- --nocapture
    cargo test --doc --workspace -- --nocapture

# Build and test components for CI including ignored tests (for GHCR environments)
ci-build-test-ghcr:
    just build-test-components
    cargo build --workspace
    cargo test --workspace -- --nocapture --include-ignored
    cargo test --doc --workspace -- --nocapture

# Display Docker cache and image information
ci-cache-info:
    docker system df
    docker images wassette-ci-*

# Clean up CI Docker images and cache
ci-clean:
    docker rmi $(docker images -q wassette-ci-* 2>/dev/null) 2>/dev/null || true
    docker builder prune -f
