# Architecture

## Module overview

```
src/
├── lib.rs        # Public library crate (re-exports all modules)
├── main.rs       # CLI entry point (clap, subcommand routing)
├── har.rs        # HAR 1.2 parsing and conversion
├── capture.rs    # Internal capture format (serialization, validation)
├── replay.rs     # Replay engine (HTTP client, concurrency, progress)
├── diff.rs       # Behavioral diff (status, headers, body, WAF detection)
├── output.rs     # Output formatting (pretty, JSON, compact, JUnit)
└── proxy.rs      # Capture proxy and remote fetch
```

## Data flow

```
HAR file ──┐
            ├──> CapturedRequest[] ──> replay() ──> ReplaySession ──> save/output
Capture  ──┘                              │
                                          │
                         ReplaySession ──>├──> diff_sessions() ──> DiffSummary ──> output
                         ReplaySession ──>┘
```

### Core types

| Type | Module | Purpose |
|------|--------|---------|
| `CapturedRequest` | `capture` | A single HTTP request to replay |
| `Capture` | `capture` | Container with version, source, and request list |
| `ReplayConfig` | `replay` | Configuration for a replay run |
| `ReplayResult` | `replay` | Result of a single replayed request |
| `ReplaySession` | `replay` | Complete replay output with metadata |
| `DiffSummary` | `diff` | Comparison result between two sessions |
| `RequestDiff` | `diff` | Per-request difference breakdown |

## Design decisions

### Library + binary split

`lib.rs` exposes all modules as a public crate. `main.rs` imports from the library. This enables:
- Integration tests that use the library API directly
- Potential embedding in other tools

### Deterministic replay

Requests replay in capture order by default (`--concurrency 1`). With `--concurrency N`, `futures::stream::buffered(N)` maintains result order while allowing N in-flight requests.

### No redirect following

`reqwest::redirect::Policy::none()` is hardcoded. Ushio records what the target *returns*, not what a browser would navigate to. This is critical for WAF comparison — a redirect to a block page is itself the signal.

### Body capture limits

Response bodies are captured up to 256 KB and only if they're valid UTF-8. Binary responses store `null` for body but still compute the SHA256 hash. This keeps session files manageable while preserving comparison capability.

### Error classification

The `classify_error()` function in `replay.rs` inspects error messages to categorize failures. This is string-based rather than type-based because `reqwest` errors are opaque after serialization through `anyhow`. The categories (Timeout, Dns, Connect, Tls, etc.) are stable enough for operator use.

### WAF detection is heuristic

`is_waf_block()` uses status codes, header prefixes, and body patterns. It will produce false positives (a 503 from an overloaded origin isn't a WAF block) and false negatives (a custom block page with no known patterns). This is by design — the diff surfaces *differences* in behavior, and operators apply judgment.

## Key dependencies

| Crate | Purpose |
|-------|---------|
| `reqwest` | HTTP client (TLS, compression, cookies, proxy) |
| `hyper` / `hyper-util` | HTTP server for the capture proxy |
| `tokio` | Async runtime |
| `futures` | Stream combinators for concurrent replay |
| `clap` / `clap_complete` | CLI parsing and shell completions |
| `similar` | Text diffing for response body comparison |
| `sha2` | SHA256 body hashing |
| `serde` / `serde_json` | Serialization for capture and session formats |
| `colored` | Terminal output formatting |
| `chrono` | Timestamps |
| `anyhow` / `thiserror` | Error handling |
| `tracing` | Structured logging |
