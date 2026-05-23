# La Chuoi - Beginner's Guide to WASM Plugins

Welcome! This guide will help you write your first WebAssembly (WASM) plugin for the La Chuoi environment. 

La Chuoi is a distributed system that runs small, sandboxed programs (plugins) on a schedule. Think of it like "Cron for WASM."

---

## 🛠️ Step 1: Prepare Your Tools

You'll need the Rust programming language installed. If you don't have it, get it at [rustup.rs](https://rustup.rs/).

Once Rust is installed, add the WebAssembly compilation target:

```bash
# We use the WASI (WebAssembly System Interface) target
rustup target add wasm32-wasip1
```

---

## 🏗️ Step 2: Create Your Project

Create a new Rust project for your plugin:

```bash
cargo new my-lachuoi-plugin
cd my-lachuoi-plugin
```

Add the following to your `Cargo.toml` to help with JSON and networking:

```toml
[dependencies]
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
# Optional: if you want to make HTTP requests
reqwest = { version = "0.12", features = ["json"] } 
tokio = { version = "1", features = ["full"] }
```

---

## 💡 Step 3: Understanding the Environment

When La Chuoi runs your plugin, it provides several **Environment Variables**:

- `APP_ID`: Your plugin's unique ID.
- `LACHUOI_TOKEN`: A temporary password your plugin uses to talk back to the Master node.
- `RPC_ENDPOINT`: The URL of the Master node's API.

---

## 🔌 Step 4: Interacting with the Host (KV Store)

La Chuoi provides a **Persistent Key-Value Store** for each plugin. You can save data (like the "last processed ID") and retrieve it later. **Keys are unique per plugin**; setting a key again will overwrite the old value.

### The "Magic" Print
The easiest way to talk to La Chuoi is by printing a JSON-RPC 2.0 message to the console. The host captures this and acts on it.

#### Saving Data (`set_kv`)
```rust
use std::env;

fn set_data(key: &str, value: &str) {
    let app_id = env::var("APP_ID").unwrap_or_else(|_| "0".to_string());
    let token = env::var("LACHUOI_TOKEN").unwrap_or_default();

    let request = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "set_kv",
        "params": {
            "task_id": app_id.parse::<i64>().unwrap_or(0),
            "token": token,
            "key": key,
            "value": value
        },
        "id": 1
    });

    // Just print it! La Chuoi will see this and save your data.
    println!("{}", request.to_string());
}
```

#### Getting Data (`get_kv`)
When you print a `get_kv` request, La Chuoi will process it and print the result back to your "Input Stream" (currently, it logs the response back to your task's log stream).

---

## 📝 Step 5: A Complete Example

Here is a simple "Counter" plugin that increments a number in the KV store every time it runs.

```rust
use std::env;
use serde_json::json;

fn main() {
    let app_id = env::var("APP_ID").unwrap_or_else(|_| "0".to_string());
    let token = env::var("LACHUOI_TOKEN").unwrap_or_default();

    println!("Hello from Plugin #{}!", app_id);

    // 1. Prepare a message to save some data
    let msg = json!({
        "jsonrpc": "2.0",
        "method": "set_kv",
        "params": {
            "task_id": app_id.parse::<i64>().unwrap_or(0),
            "token": token,
            "key": "last_run",
            "value": chrono::Utc::now().to_rfc3339()
        },
        "id": 1
    });

    // 2. Print it to communicate with the host
    println!("{}", msg.to_string());
    
    println!("Task completed successfully!");
}
```

---

## 🚀 Step 6: Build and Deploy

### 1. Compile to WASM
Run this command in your project folder:
```bash
cargo build --target wasm32-wasip1 --release
```

### 2. Copy to La Chuoi
Find the generated file at `target/wasm32-wasip1/release/my_lachuoi_plugin.wasm` and copy it into the `plugins/` directory of your La Chuoi installation.

### 3. Add to `cron.toml`
Open `cron.toml` and add a new task:

```toml
[[task]]
name = "My First Plugin"
cron = "0 */5 * * * *" # Every 5 minutes
type = "wasm"
payload = "my_lachuoi_plugin.wasm"
```

### 4. Update Checksums
Back in the La Chuoi directory, run the helper command to register your new binary:

```bash
just update-plugins
```

This will automatically add the `sha256sum` to your `cron.toml`.

---

## ⚖️ Constraints & Best Practices

- **Sandboxing**: Your plugin cannot access files on the host computer or see other plugins' data.
- **Networking**: If your plugin is a **WASM Component (Preview 2)**, it has outbound HTTP access enabled. 
- **Integrity**: Always use `just update-plugins`. La Chuoi will refuse to run any plugin whose `sha256sum` doesn't match what is in `cron.toml`.
- **Logs**: Everything you print to `stdout` or `stderr` will appear in the La Chuoi Dashboard. Keep it clean!
