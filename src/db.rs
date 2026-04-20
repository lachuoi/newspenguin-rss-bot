use crate::wasi_http::http_request;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::env;
use wasi as bindings;

#[derive(Serialize)]
struct Value {
    #[serde(rename = "type")]
    value_type: String,
    value: String,
}

#[derive(Serialize)]
struct Stmt {
    sql: String,
    args: Vec<Value>,
}

#[derive(Serialize)]
struct Request {
    #[serde(rename = "type")]
    req_type: String,
    stmt: Stmt,
}

#[derive(Serialize)]
struct Pipeline {
    requests: Vec<Request>,
}

#[derive(Deserialize)]
struct PipelineResponse {
    results: Vec<serde_json::Value>,
}

async fn execute_sql(
    sql: String,
    args: Vec<Value>,
) -> Result<serde_json::Value> {
    let url = env::var("LIBSQL_URL").expect("LIBSQL_URL not set");
    let token = env::var("LIBSQL_TOKEN").expect("LIBSQL_TOKEN not set");

    let pipeline = Pipeline {
        requests: vec![
            Request {
                req_type: "execute".to_string(),
                stmt: Stmt { sql, args },
            },
            Request {
                req_type: "close".to_string(),
                stmt: Stmt {
                    sql: "".to_string(),
                    args: vec![],
                },
            },
        ],
    };

    let body = serde_json::to_vec(&pipeline)?;
    let headers = vec![
        (
            "Authorization".to_string(),
            format!("Bearer {}", token).into_bytes(),
        ),
        (
            "Content-Type".to_string(),
            "application/json".to_string().into_bytes(),
        ),
    ];

    let resp_body = http_request(
        bindings::http::types::Method::Post,
        &format!("{}/v2/pipeline", url.trim_end_matches('/')),
        headers,
        Some(body),
    )
    .await?;

    let resp: PipelineResponse = serde_json::from_slice(&resp_body)?;
    let result = resp
        .results
        .get(0)
        .ok_or_else(|| anyhow::anyhow!("No results in pipeline response"))?;

    if let Some(error) = result.get("error") {
        return Err(anyhow::anyhow!("Turso error: {}", error));
    }

    let response = result
        .get("response")
        .ok_or_else(|| anyhow::anyhow!("No response in pipeline result"))?;
    Ok(response.clone())
}

pub async fn get_kv(key: &str) -> Result<Option<String>> {
    let table_name = env::var("LIBSQL_KV_TABLE").unwrap_or_else(|_| "kv_store".to_string());

    // Ensure table exists
    let _ = execute_sql(
        format!("CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value TEXT)", table_name),
        vec![],
    )
    .await?;

    let resp = execute_sql(
        format!("SELECT value FROM {} WHERE key = ?", table_name),
        vec![Value {
            value_type: "text".to_string(),
            value: key.to_string(),
        }],
    )
    .await?;

    let val = resp.pointer("/result/rows/0/0/value");
    
    match val {
        Some(serde_json::Value::String(s)) => Ok(Some(s.clone())),
        _ => Ok(None),
    }
}

pub async fn set_kv(key: &str, value: &str) -> Result<()> {
    let table_name = env::var("LIBSQL_KV_TABLE").unwrap_or_else(|_| "kv_store".to_string());

    // Ensure table exists
    let _ = execute_sql(
        format!("CREATE TABLE IF NOT EXISTS {} (key TEXT PRIMARY KEY, value TEXT)", table_name),
        vec![],
    )
    .await?;

    execute_sql(
        format!("INSERT INTO {} (key, value) VALUES (?, ?) ON CONFLICT(key) DO UPDATE SET value = excluded.value", table_name),
        vec![
            Value {
                value_type: "text".to_string(),
                value: key.to_string(),
            },
            Value {
                value_type: "text".to_string(),
                value: value.to_string(),
            },
        ],
    )
    .await?;
    
    Ok(())
}
