# Design Document: NewsPenguin RSS Bot

This document outlines the architecture, data flow, and persistence strategy of the NewsPenguin RSS Bot.

## Overview

The NewsPenguin RSS Bot is a lightweight WebAssembly (WASI) component designed to monitor the NewsPenguin RSS feed and post new articles to Mastodon. It is optimized for the La Chuoi distributed runtime.

## Architecture

### WASI Component Model
The bot is built as a **WASI Preview 2** component. This ensures:
- **Portability**: Runs on any WASI-compliant runtime (Wasmtime, La Chuoi).
- **Security**: Strict capability-based security (network and environment access must be explicitly granted).
- **Efficiency**: Low cold-start latency and minimal resource footprint.

### Core Modules
- `main.rs`: Orchestrates the RSS fetching, parsing, and posting logic.
- `db.rs`: Handles persistence via La Chuoi JSON-RPC or local fallback.
- `wasi_http.rs`: Provides a low-level wrapper around WASI 0.2 HTTP types.

## Persistence Model

### La Chuoi KV Store
The bot uses a persistent Key-Value store provided by the La Chuoi runtime for state management.

#### 1. Last Processed Timestamp
- **Key**: `newspenguin-rss.last_build_date`
- **Value**: The UTC timestamp of the last successful run.
- **Purpose**: To filter out articles that are older than the last execution.

#### 2. Duplicate Detection (Duplicate Keys)
The bot leverages La Chuoi's support for **duplicate keys** to track post history.
- **Key**: `posted link`
- **Value**: `<URL>` (multiple entries allowed)
- **Logic**: When an article is processed, the bot checks if its link exists in the list of values for the `posted link` key. If not found, the article is posted and the link is appended to the KV store.

### Local Fallback
When running outside of a La Chuoi environment (e.g., local development), the bot falls back to `storage.json`. This file mimics the duplicate key behavior by storing keys as arrays of strings.

## Data Flow

1.  **Fetch**: Fetch the RSS feed using `wasi-http`.
2.  **Filter**:
    -   Discard articles older than 2 hours.
    -   Discard articles older than the `last_build_date`.
3.  **Check Duplicates**: Query the KV store for the article link under the `posted link` key.
4.  **Post**: If new, post the article to Mastodon using its REST API.
5.  **Persist**: 
    -   Append the link to the `posted link` key.
    -   Update `last_build_date` to the current time.

## Configuration

Configuration is managed via environment variables provided by the host environment or a `.env` file:
- `RPC_ENDPOINT`: Gateway to the La Chuoi control plane.
- `NEWSPENGUIN_MSTD_ACCESS_TOKEN`: Mastodon API authentication.
- `ENVIRONMENT`: Controls behavior (e.g., `development` disables actual posting).

## Security Considerations

- **Secrets**: Mastodon tokens and La Chuoi tokens are never logged.
- **Isolation**: The WASM sandbox prevents the bot from accessing the host's filesystem or internal network beyond what is explicitly allowed.
