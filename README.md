# NewsPenguin RSS Bot

A WASI-based bot that monitors the NewsPenguin RSS feed and posts new articles to a Mastodon instance.

## Features

- **WASI Component**: Built as a WebAssembly component using `cargo-component` and WASI 0.2.
- **RSS Monitoring**: Fetches and parses RSS feeds (defaulting to NewsPenguin).
- **Persistence**: Tracks the last processed article date and posted links using La Chuoi JSON-RPC KV store. It utilizes the runtime's duplicate key support to maintain a history of all posted articles under a single `posted link` key.
- **Mastodon Integration**: Automatically posts new articles to a configured Mastodon account with public visibility.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [cargo-component](https://github.com/bytecodealliance/cargo-component)
- [wasmtime](https://wasmtime.dev/) (to run the component)
- [just](https://github.com/casey/just) (optional, for running tasks)

## Configuration

The bot is configured via environment variables. Ensure these are set in your host environment before running.

| Variable | Description | Default / Example |
|----------|-------------|-------------------|
| `APP_ID` | Unique numeric ID of the task | **Provided by Runtime** |
| `LACHUOI_TOKEN` | Auth token for system RPC calls | **Provided by Runtime** |
| `RPC_ENDPOINT` | HTTP URI of the JSON-RPC service | **Provided by Runtime** |
| `NEWSPENGUIN_MSTD_ACCESS_TOKEN` | Mastodon API access token | **Required** |
| `NEWSPENGUIN_MSTD_API_URI` | Mastodon instance URL | `https://mstd.seungjin.net` |
| `NEWSPENGUIN_RSS_URI` | RSS feed URL | `https://www.newspenguin.com/rss/allArticle.xml` |
| `NEWSPENGUIN_USER_AGENT` | Custom User-Agent header | (Optional) |

## Usage

### Building

To build the WebAssembly component:

```bash
just build
```

Or using `cargo-component` directly:

```bash
cargo component build --target wasm32-wasip2
```

### Running

To run the bot locally using `wasmtime` (ensure environment variables are exported):

```bash
just run
# or
just run-release
```

This command enables the necessary WASI features (HTTP, network, environment inheritance).

### Standalone Mode

The bot can be run standalone without a La Chuoi server. If the `RPC_ENDPOINT` environment variable is not set, the bot will automatically fall back to using a local `storage.json` file in the current directory for persistence.

### Environment Behavior

- **Production**: Default behavior. Posts to Mastodon and uses RPC for persistence if available.
- **Development**: If `ENVIRONMENT=development` is set, the bot will log messages to the console but **will not** post to Mastodon.

### Deployment

The project includes a `Dockerfile` to build the `.wasm` component in a containerized environment.

## Links

- [NewsPenguin RSS Index](https://www.newspenguin.com/rssIndex.html)
- [Bot Account on Mastodon](https://mstd.seungjin.net/@newspenguin)
- [Github Mirror](https://github.com/lachuoi/newspenguin-rss-bot)

## License

This project is dual-licensed under the MIT License and the Apache License (Version 2.0).

- See [LICENSE-MIT](LICENSE-MIT) for details.
- See [LICENSE-APACHE](LICENSE-APACHE) for details.