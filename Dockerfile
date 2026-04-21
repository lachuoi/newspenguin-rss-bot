# Use a recent Rust version that supports wasm32-wasip2
FROM rust:1.80-slim AS builder

# Install cargo-component
RUN apt-get update && apt-get install -y pkg-config libssl-dev curl && \
    curl -L https://github.com/bytecodealliance/cargo-component/releases/download/v0.18.0/cargo-component-v0.18.0-x86_64-unknown-linux-musl.tar.gz | tar xz -C /usr/local/bin --strip-components=1 && \
    rustup target add wasm32-wasip2

WORKDIR /usr/src/app
COPY . .

# Build the component
RUN cargo component build --release --target wasm32-wasip2

# Use a minimal runtime image or just keep the wasm file
FROM alpine:latest
RUN apk add --no-cache ca-certificates
WORKDIR /app
COPY --from=builder /usr/src/app/target/wasm32-wasip2/release/newspenguin-rss-bot.wasm .

# In WASI, we don't "run" the binary directly in the container unless we have a runner like wasmtime
# This Dockerfile just builds and provides the .wasm file.
# If you want to RUN it, you'd need wasmtime installed.
CMD ["cp", "newspenguin-rss-bot.wasm", "/output/"]
