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

fn load_local_storage() -> HashMap<String, String> {
    if let Ok(content) = fs::read_to_string(LOCAL_STORAGE_FILE) {
        serde_json::from_str(&content).unwrap_or_default()
    } else {
        HashMap::new()
    }
}

fn save_local_storage(storage: &HashMap<String, String>) -> Result<()> {
    let content = serde_json::to_string_pretty(storage)?;
    fs::write(LOCAL_STORAGE_FILE, content)?;
    Ok(())
}

pub async fn get_kv(_app_id: i64, key: &str) -> Result<Option<String>> {
    if env::var("RPC_ENDPOINT").is_err() {
        let storage = load_local_storage();
        return Ok(storage.get(key).cloned());
    }

    let resp = call_rpc("kv_get", json!({ "key": key })).await?;
    match resp {
        serde_json::Value::String(s) => Ok(Some(s)),
        serde_json::Value::Null => Ok(None),
        _ => Ok(Some(resp.to_string())),
    }
}

pub async fn set_kv(_app_id: i64, key: &str, value: &str) -> Result<()> {
    if env::var("RPC_ENDPOINT").is_err() {
        let mut storage = load_local_storage();
        storage.insert(key.to_string(), value.to_string());
        return save_local_storage(&storage);
    }

    call_rpc("kv_set", json!({ "key": key, "value": value })).await?;
    Ok(())
}

pub async fn check_link_published(_app_id: i64, link: &str) -> Result<bool> {
    let key = format!("link:{}", link);
    let val = get_kv(0, &key).await?;
    Ok(val.is_some())
}

pub async fn add_posted_link(_app_id: i64, link: &str) -> Result<()> {
    let key = format!("link:{}", link);
    set_kv(0, &key, "1").await?;
    Ok(())
}

pub async fn delete_old_posted_messages(_app_id: i64) -> Result<()> {
    // La Chuoi RPC does not currently support bulk deletion or listing.
    // Old links will remain in the KV store for now.
    Ok(())
}

// execute_sql is removed as it's not supported by La Chuoi RPC directly.
// If needed, it should be replaced by high-level KV operations.
