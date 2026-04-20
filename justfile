# Default task
default: help

# Show available tasks
help:
    @just --list

# Check the project for errors
check:
    cargo check --target wasm32-wasip2

# Build the WebAssembly component
build:
    cargo build --target wasm32-wasip2

# Build the component in release mode
build-release:
    cargo build --target wasm32-wasip2 --release

# Clean the build artifacts
clean:
    cargo clean

# Run the project (example using wasmtime, adjust if using cargo-component or spin)
run: build
    wasmtime run -S http -S inherit-network=y -S allow-ip-name-lookup=y --dir . ./target/wasm32-wasip2/debug/wasi-newspenguin-rss-bot.wasm

# Run the project in release mode
run-release: build-release
    wasmtime run -S http -S inherit-network=y -S allow-ip-name-lookup=y --dir . ./target/wasm32-wasip2/release/wasi-newspenguin-rss-bot.wasm
