# Default task
default: help

# Show available tasks
help:
    @just --list

# Check the project for errors
check:
    cargo component check --target wasm32-wasip2

# Build the WebAssembly component
build flags="":
    cargo component build --target wasm32-wasip2 {{flags}}

# Build the component in release mode
build-release: (build "--release")

# Clean the build artifacts
clean:
    cargo clean

# Run the project
run flags="":
    @just build $(if echo "{{flags}}" | grep -q "\--release"; then echo "--release"; else echo ""; fi)
    @wasmtime run \
        --dir . \
        -S http \
        -S inherit-network=y \
        -S allow-ip-name-lookup=y \
        -S inherit-env=y \
        ./target/wasm32-wasip2/$(if echo "{{flags}}" | grep -q "\--release"; then echo "release"; else echo "debug"; fi)/newspenguin-rss-bot.wasm \
        {{flags}}




# Run the project in release mode
run-release: (run "--release")

# Delete Mastodon posts from the last 3 hours
cleanup-mastodon:
    @./delete_recent_posts.sh
