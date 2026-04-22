//! Capture proxy and remote fetch
//!
//! Provides a reverse proxy that records traffic into ushio capture format,
//! and a client for fetching request logs from remote endpoints (e.g. Sentinel).

use anyhow::{Context, Result};
use http_body_util::{BodyExt, Full};
use hyper::body::Bytes;
use hyper::server::conn::http1;
use hyper::service::service_fn;
use hyper::{Request, Response};
use hyper_util::rt::TokioIo;
use std::net::SocketAddr;
use std::sync::{Arc, Mutex};
use tokio::net::TcpListener;

use crate::capture::{Capture, CapturedRequest};

/// Run a reverse proxy that records all traffic to a capture file.
///
/// Listens on `listen_addr`, forwards requests to `target_url`, and
/// saves all request/response pairs to `output_path` on shutdown (Ctrl-C).
pub async fn run_capture_proxy(
    listen_addr: &str,
    target_url: &str,
    output_path: &str,
    insecure: bool,
) -> Result<()> {
    let addr: SocketAddr = listen_addr
        .parse()
        .context("Invalid listen address (expected host:port, e.g. 0.0.0.0:8080)")?;

    let listener = TcpListener::bind(addr)
        .await
        .context(format!("Failed to bind to {}", addr))?;

    eprintln!("Capture proxy listening on {}", addr);
    eprintln!("Forwarding to {}", target_url);
    eprintln!("Press Ctrl-C to stop and save capture");

    let target = target_url.to_string();
    let requests: Arc<Mutex<Vec<CapturedRequest>>> = Arc::new(Mutex::new(Vec::new()));
    let output = output_path.to_string();

    // Build reqwest client for forwarding
    let mut client_builder = reqwest::Client::builder().redirect(reqwest::redirect::Policy::none());
    if insecure {
        client_builder = client_builder.danger_accept_invalid_certs(true);
    }
    let client = Arc::new(
        client_builder
            .build()
            .context("Failed to build HTTP client")?,
    );

    // Spawn Ctrl-C handler to save on shutdown
    let requests_clone = requests.clone();
    let output_clone = output.clone();
    let target_clone = target.clone();
    tokio::spawn(async move {
        tokio::signal::ctrl_c().await.ok();
        let reqs = requests_clone.lock().unwrap();
        let capture = Capture::new(reqs.clone()).with_source(format!("proxy:{}", target_clone));
        let json = serde_json::to_string_pretty(&capture).unwrap_or_default();
        if let Err(e) = std::fs::write(&output_clone, &json) {
            eprintln!("\nFailed to write capture: {}", e);
        } else {
            eprintln!("\nSaved {} requests to {}", reqs.len(), output_clone);
        }
        std::process::exit(0);
    });

    loop {
        let (stream, remote_addr) = listener.accept().await?;
        let io = TokioIo::new(stream);
        let target = target.clone();
        let requests = requests.clone();
        let client = client.clone();

        tokio::spawn(async move {
            let service = service_fn(move |req: Request<hyper::body::Incoming>| {
                let target = target.clone();
                let requests = requests.clone();
                let client = client.clone();
                async move { handle_proxy_request(req, &target, &requests, &client, remote_addr).await }
            });

            if let Err(e) = http1::Builder::new().serve_connection(io, service).await {
                if !e.to_string().contains("connection closed") {
                    eprintln!("Connection error from {}: {}", remote_addr, e);
                }
            }
        });
    }
}

/// Handle a single proxied request
async fn handle_proxy_request(
    req: Request<hyper::body::Incoming>,
    target: &str,
    requests: &Arc<Mutex<Vec<CapturedRequest>>>,
    client: &reqwest::Client,
    _remote_addr: SocketAddr,
) -> Result<Response<Full<Bytes>>, hyper::Error> {
    let method = req.method().to_string();
    let path = req
        .uri()
        .path_and_query()
        .map(|pq| pq.as_str())
        .unwrap_or("/");
    let forward_url = format!("{}{}", target.trim_end_matches('/'), path);

    // Collect request headers
    let req_headers: Vec<(String, String)> = req
        .headers()
        .iter()
        .filter(|(name, _)| {
            let n = name.as_str().to_lowercase();
            n != "host" && n != "content-length"
        })
        .map(|(k, v)| (k.to_string(), v.to_str().unwrap_or("").to_string()))
        .collect();

    // Read request body
    let body_bytes = req
        .collect()
        .await
        .map(|b| b.to_bytes())
        .unwrap_or_default();
    let req_body = if body_bytes.is_empty() {
        None
    } else {
        String::from_utf8(body_bytes.to_vec()).ok()
    };

    // Forward the request
    let reqwest_method: reqwest::Method = method.parse().unwrap_or(reqwest::Method::GET);
    let mut forward = client.request(reqwest_method, &forward_url);
    for (k, v) in &req_headers {
        forward = forward.header(k.as_str(), v.as_str());
    }
    if let Some(ref body) = req_body {
        forward = forward.body(body.clone());
    }

    match forward.send().await {
        Ok(resp) => {
            let status = resp.status().as_u16();

            // Record the request
            {
                let mut reqs = requests.lock().unwrap();
                reqs.push(CapturedRequest {
                    method: method.clone(),
                    url: forward_url.clone(),
                    headers: req_headers,
                    body: req_body,
                    expected_status: Some(status),
                });
                if reqs.len() % 10 == 0 {
                    eprint!("\r  Captured {} requests", reqs.len());
                }
            }

            // Build response back to client
            let resp_status = hyper::StatusCode::from_u16(status)
                .unwrap_or(hyper::StatusCode::INTERNAL_SERVER_ERROR);
            let resp_body = resp.bytes().await.unwrap_or_default();

            let mut response = Response::builder().status(resp_status);
            // We skip forwarding response headers for simplicity — the capture
            // records the request side, which is what ushio replays.
            response = response.header("x-ushio-proxy", "true");

            Ok(response.body(Full::new(resp_body)).unwrap())
        }
        Err(e) => {
            eprintln!("  Forward error for {} {}: {}", method, forward_url, e);
            Ok(Response::builder()
                .status(502)
                .body(Full::new(Bytes::from(format!("Proxy error: {}", e))))
                .unwrap())
        }
    }
}

/// Fetch request logs from a remote URL.
///
/// Expects the endpoint to return JSON in one of these formats:
///
/// 1. Ushio capture format: `{ "version": "1.0", "requests": [...] }`
/// 2. Array of requests: `[{ "method": "GET", "url": "...", ... }]`
/// 3. Sentinel log format: `{ "entries": [{ "method": "GET", "url": "...", ... }] }`
pub async fn fetch_remote_capture(url: &str, insecure: bool) -> Result<Vec<CapturedRequest>> {
    let mut builder = reqwest::Client::builder();
    if insecure {
        builder = builder.danger_accept_invalid_certs(true);
    }
    let client = builder.build().context("Failed to build HTTP client")?;

    let resp = client
        .get(url)
        .header("Accept", "application/json")
        .send()
        .await
        .context(format!("Failed to fetch from {}", url))?;

    if !resp.status().is_success() {
        anyhow::bail!("Remote returned status {}", resp.status());
    }

    let text = resp.text().await.context("Failed to read response body")?;

    // Try ushio capture format
    if let Ok(capture) = serde_json::from_str::<Capture>(&text) {
        return Ok(capture.requests);
    }

    // Try plain array of requests
    if let Ok(requests) = serde_json::from_str::<Vec<CapturedRequest>>(&text) {
        return Ok(requests);
    }

    // Try Sentinel-style { "entries": [...] }
    #[derive(serde::Deserialize)]
    struct EntriesWrapper {
        entries: Vec<CapturedRequest>,
    }
    if let Ok(wrapper) = serde_json::from_str::<EntriesWrapper>(&text) {
        return Ok(wrapper.entries);
    }

    anyhow::bail!(
        "Could not parse response from {} as ushio capture, request array, or entries wrapper",
        url
    )
}
