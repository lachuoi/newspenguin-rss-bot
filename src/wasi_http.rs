// Copyright 2026 Seungjin Kim
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use wasi as bindings;
use anyhow::Result;

pub async fn http_request(
    method: bindings::http::types::Method,
    url: &str,
    headers: Vec<(String, Vec<u8>)>,
    body: Option<Vec<u8>>,
) -> Result<Vec<u8>> {
    use bindings::http::types::{OutgoingRequest, Fields, Scheme, OutgoingBody};
    use bindings::http::outgoing_handler::handle;

    let parsed_url = url::Url::parse(url)?;
    let scheme = match parsed_url.scheme() {
        "http" => Scheme::Http,
        "https" => Scheme::Https,
        _ => Scheme::Other(parsed_url.scheme().to_string()),
    };

    let request_headers = Fields::new();
    for (k, v) in headers {
        request_headers.set(&k, &[v]).map_err(|_| anyhow::anyhow!("failed to set header {}", k))?;
    }

    let path = parsed_url.path();
    let query = parsed_url.query();
    let path_with_query = if let Some(q) = query {
        format!("{}?{}", path, q)
    } else {
        path.to_string()
    };

    let request = OutgoingRequest::new(request_headers);
    request.set_method(&method).map_err(|_| anyhow::anyhow!("failed to set method"))?;
    request.set_scheme(Some(&scheme)).map_err(|_| anyhow::anyhow!("failed to set scheme"))?;
    request.set_authority(Some(parsed_url.host_str().unwrap())).map_err(|_| anyhow::anyhow!("failed to set authority"))?;
    request.set_path_with_query(Some(&path_with_query)).map_err(|_| anyhow::anyhow!("failed to set path"))?;

    if let Some(b) = body {
        let outgoing_body = request.body().map_err(|_| anyhow::anyhow!("failed to get body"))?;
        let stream = outgoing_body.write().map_err(|_| anyhow::anyhow!("failed to get stream"))?;
        stream.blocking_write_and_flush(&b).map_err(|_| anyhow::anyhow!("failed to write body"))?;
        drop(stream);
        OutgoingBody::finish(outgoing_body, None).map_err(|_| anyhow::anyhow!("failed to finish body"))?;
    }

    let future_response = handle(request, None).map_err(|e| anyhow::anyhow!("failed to send request: {:?}", e))?;
    
    // Poll for the response
    let pollable = future_response.subscribe();
    loop {
        if let Some(result) = future_response.get() {
            let response = result.map_err(|_| anyhow::anyhow!("request failed"))?
                .map_err(|_| anyhow::anyhow!("HTTP error"))?;
            
            let status = response.status();
            if status < 200 || status >= 300 {
                let mut error_body = String::new();
                if let Ok(body) = response.consume() {
                    if let Ok(stream) = body.stream() {
                        if let Ok(data) = stream.blocking_read(1024) {
                            error_body = String::from_utf8_lossy(&data).to_string();
                        }
                    }
                }
                return Err(anyhow::anyhow!("HTTP status {}: {}", status, error_body));
            }

            let body = response.consume().map_err(|_| anyhow::anyhow!("failed to consume response"))?;
            let stream = body.stream().map_err(|_| anyhow::anyhow!("failed to get response stream"))?;
            
            let mut buf = Vec::new();
            loop {
                let chunk = stream.blocking_read(1024 * 64);
                match chunk {
                    Ok(data) => {
                        if data.is_empty() {
                            break;
                        }
                        buf.extend_from_slice(&data);
                    }
                    Err(bindings::io::streams::StreamError::Closed) => break,
                    Err(e) => return Err(anyhow::anyhow!("stream error: {:?}", e)),
                }
            }
            return Ok(buf);
        }
        pollable.block();
    }
}
