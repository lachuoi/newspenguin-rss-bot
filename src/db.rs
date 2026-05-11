// Copyright 2026 Seungjin Kim
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use crate::wasi_http::http_request;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::collections::HashMap;
use std::env;
use std::fs;
use wasi as bindings;

#[derive(Serialize)]
struct JsonRpcRequest {
    jsonrpc: String,
    method: String,
    params: serde_json::Value,
    id: i32,
}

#[derive(Deserialize)]
struct JsonRpcResponse {
    result: Option<serde_json::Value>,
    error: Option<serde_json::Value>,
}

async fn call_rpc(
    method: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let endpoint = env::var("RPC_ENDPOINT")
        .map_err(|_| anyhow::anyhow!("RPC_ENDPOINT not set"))?;
    let token = env::var("LACHUOI_TOKEN")
        .map_err(|_| anyhow::anyhow!("LACHUOI_TOKEN not set"))?;
    let app_id = env::var("APP_ID")
        .map_err(|_| anyhow::anyhow!("APP_ID not set"))?
        .parse::<i64>()?;

    let mut rpc_params = params;
    if let Some(obj) = rpc_params.as_object_mut() {
        obj.insert("token".to_string(), json!(token));
        obj.insert("task_id".to_string(), json!(app_id));
    }

    let request = JsonRpcRequest {
        jsonrpc: "2.0".to_string(),
        method: method.to_string(),
        params: rpc_params,
        id: 1,
    };

    let body = serde_json::to_vec(&request)?;
    let headers = vec![(
        "Content-Type".to_string(),
        "application/json".to_string().into_bytes(),
    )];

    let resp_body = http_request(
        bindings::http::types::Method::Post,
        &endpoint,
        headers,
        Some(body),
    )
    .await?;

    let resp: JsonRpcResponse = serde_json::from_slice(&resp_body)?;
    if let Some(error) = resp.error {
        return Err(anyhow::anyhow!("JSON-RPC error: {}", error));
    }

    resp.result
        .ok_or_else(|| anyhow::anyhow!("Missing result in JSON-RPC response"))
}

const LOCAL_STORAGE_FILE: &str = "storage.json";

fn load_local_storage() -> HashMap<String, Vec<String>> {
    if let Ok(content) = fs::read_to_string(LOCAL_STORAGE_FILE) {
        if let Ok(storage) =
            serde_json::from_str::<HashMap<String, Vec<String>>>(&content)
        {
            return storage;
        }
        // Fallback and migration from old format
        if let Ok(old_storage) =
            serde_json::from_str::<HashMap<String, String>>(&content)
        {
            let mut new_storage: HashMap<String, Vec<String>> = HashMap::new();
            for (k, v) in old_storage {
                if k.starts_with("link:") {
                    new_storage
                        .entry("posted link".to_string())
                        .or_default()
                        .push(k);
                } else {
                    new_storage.insert(k, vec![v]);
                }
            }
            return new_storage;
        }
    }
    HashMap::new()
}

fn save_local_storage(storage: &HashMap<String, Vec<String>>) -> Result<()> {
    let content = serde_json::to_string_pretty(storage)?;
    fs::write(LOCAL_STORAGE_FILE, content)?;
    Ok(())
}

pub async fn get_kv_list(_app_id: i64, key: &str) -> Result<Vec<String>> {
    if env::var("RPC_ENDPOINT").is_err() {
        let storage = load_local_storage();
        return Ok(storage.get(key).cloned().unwrap_or_default());
    }

    let resp = call_rpc("get_key", json!({ "key": key })).await?;
    match resp {
        serde_json::Value::Array(arr) => {
            let res = arr
                .into_iter()
                .map(|v| {
                    v.as_str()
                        .map(|s| s.to_string())
                        .unwrap_or_else(|| v.to_string())
                })
                .collect();
            Ok(res)
        }
        serde_json::Value::String(s) => Ok(vec![s]),
        serde_json::Value::Null => Ok(vec![]),
        _ => Ok(vec![resp.to_string()]),
    }
}

pub async fn get_kv(app_id: i64, key: &str) -> Result<Option<String>> {
    let list = get_kv_list(app_id, key).await?;
    Ok(list.last().cloned())
}

pub async fn set_kv(_app_id: i64, key: &str, value: &str) -> Result<()> {
    if env::var("RPC_ENDPOINT").is_err() {
        let mut storage = load_local_storage();
        storage.entry(key.to_string()).or_default().push(value.to_string());
        return save_local_storage(&storage);
    }

    call_rpc("set_key", json!({ "key": key, "value": value })).await?;
    Ok(())
}

pub async fn check_link_published(app_id: i64, link: &str) -> Result<bool> {
    let links = get_kv_list(app_id, "posted link").await?;
    let target = format!("link:{}", link);
    Ok(links.contains(&target))
}

pub async fn add_posted_link(app_id: i64, link: &str) -> Result<()> {
    let value = format!("link:{}", link);
    set_kv(app_id, "posted link", &value).await?;
    Ok(())
}

pub async fn delete_old_posted_messages(_app_id: i64) -> Result<()> {
    // La Chuoi RPC does not currently support bulk deletion or listing.
    // Old links will remain in the KV store for now.
    Ok(())
}

// execute_sql is removed as it's not supported by La Chuoi RPC directly.
// If needed, it should be replaced by high-level KV operations.
