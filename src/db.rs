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
        if let Ok(storage) =
            serde_json::from_str::<HashMap<String, String>>(&content)
        {
            return storage;
        }
    }
    HashMap::new()
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

    let resp = call_rpc("get_kv", json!({ "key": key })).await?;
    match resp {
        serde_json::Value::String(s) => Ok(Some(s)),
        _ => Ok(None),
    }
}

pub async fn set_kv(_app_id: i64, key: &str, value: &str) -> Result<()> {
    if env::var("RPC_ENDPOINT").is_err() {
        let mut storage = load_local_storage();
        storage.insert(key.to_string(), value.to_string());
        return save_local_storage(&storage);
    }

    call_rpc("set_kv", json!({ "key": key, "value": value })).await?;
    Ok(())
}

pub async fn delete_kv(_app_id: i64, key: &str) -> Result<()> {
    if env::var("RPC_ENDPOINT").is_err() {
        let mut storage = load_local_storage();
        storage.remove(key);
        return save_local_storage(&storage);
    }

    call_rpc("set_kv", json!({ "key": key, "value": serde_json::Value::Null })).await?;
    Ok(())
}

pub async fn list_kv(_app_id: i64) -> Result<HashMap<String, String>> {
    if env::var("RPC_ENDPOINT").is_err() {
        return Ok(load_local_storage());
    }

    let resp = call_rpc("get_kv", json!({ "key": serde_json::Value::Null })).await?;
    match resp {
        serde_json::Value::Object(map) => {
            let mut res = HashMap::new();
            for (k, v) in map {
                if let Some(s) = v.as_str() {
                    res.insert(k, s.to_string());
                }
            }
            Ok(res)
        }
        _ => Ok(HashMap::new()),
    }
}

pub async fn check_link_published(app_id: i64, link: &str) -> Result<bool> {
    // Unique key per link
    let key = format!("link:{}", link);
    Ok(get_kv(app_id, &key).await?.is_some())
}

pub async fn add_posted_link(app_id: i64, link: &str) -> Result<()> {
    let key = format!("link:{}", link);
    let now = chrono::Utc::now().to_rfc3339();
    set_kv(app_id, &key, &now).await?;
    Ok(())
}

pub async fn delete_old_posted_messages(app_id: i64) -> Result<()> {
    let all = list_kv(app_id).await?;
    let now = chrono::Utc::now();
    let week_ago = now - chrono::Duration::days(7);

    for (k, v) in all {
        if k.starts_with("link:") {
            if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(&v) {
                if dt.with_timezone(&chrono::Utc) < week_ago {
                    let _ = delete_kv(app_id, &k).await;
                }
            }
        }
    }
    Ok(())
}
