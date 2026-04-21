# NewsPenguin RSS Bot

A WASI-based bot that monitors the NewsPenguin RSS feed and posts new articles to a Mastodon instance.

## Features

- **WASI Component**: Built as a WebAssembly component using `cargo-component` and WASI 0.2.
- **RSS Monitoring**: Fetches and parses RSS feeds (defaulting to NewsPenguin).
- **Persistence**: Tracks the last processed article date using Turso (libSQL) to avoid duplicate posts.
- **Mastodon Integration**: Automatically posts new articles to a configured Mastodon account with private visibility by default.

## Prerequisites

- [Rust](https://www.rust-lang.org/tools/install) (latest stable)
- [cargo-component](https://github.com/bytecodealliance/cargo-component)
- [wasmtime](https://wasmtime.dev/) (to run the component)
- [just](https://github.com/casey/just) (optional, for running tasks)

## Configuration

The bot is configured via environment variables. You can create a `.env` file in the root directory.

| Variable | Description | Default / Example |
|----------|-------------|-------------------|
| `NEWSPENGUIN_MSTD_ACCESS_TOKEN` | Mastodon API access token | **Required** |
| `NEWSPENGUIN_MSTD_API_URI` | Mastodon instance URL | `https://mstd.seungjin.net` |
| `NEWSPENGUIN_RSS_URI` | RSS feed URL | `https://www.newspenguin.com/rss/allArticle.xml` |
| `LIBSQL_URL` | Turso/libSQL database URL | `libsql://your-db.turso.io` |
| `LIBSQL_TOKEN` | Turso/libSQL auth token | **Required** |
| `LIBSQL_KV_TABLE` | Table name for KV storage | `kv_store` |

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

To run the bot locally using `wasmtime`:

```bash
just run
```

This command enables the necessary WASI features (HTTP, network) and mounts the current directory for `.env` access.

### Deployment

The project includes a `Dockerfile` to build the `.wasm` component in a containerized environment.

## Links

- [NewsPenguin RSS Index](https://www.newspenguin.com/rssIndex.html)
- [Bot Account on Mastodon](https://mstd.seungjin.net/@newspenguin)
- [Github Mirror](https://github.com/lachuoi/newspenguin-rss-bot)