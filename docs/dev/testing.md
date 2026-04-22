# Testing

## Test structure

Tests are split between unit tests (in-module) and integration tests (separate binary):

| Location | Count | What's tested |
|----------|-------|--------------|
| `src/replay.rs` | 5 | URL rewriting, header mutations, cookie stripping |
| `src/diff.rs` | 11 | Status diff, body diff, WAF detection (headers + body patterns) |
| `tests/integration.rs` | 20 | End-to-end: HAR parsing, capture round-trips, replay, diff, new features |

## Unit tests

Unit tests are embedded in each module under `#[cfg(test)]`. They test internal logic without network access.

```bash
cargo test --lib
```

### diff.rs tests

- `test_diff_identical` — two identical results produce no diff
- `test_diff_status` — status code difference is detected
- `test_waf_block_detection` — 403 + WAF header is flagged as block
- `test_waf_diff` — allowed vs blocked produces a WafDiff
- `test_body_diff_identical` — same body = no diff
- `test_body_diff_different` — different body produces unified diff
- `test_body_diff_one_missing` — one body present, one missing = diff
- `test_waf_block_body_cloudflare` — Cloudflare block page detected in body
- `test_waf_block_body_generic` — "Access Denied" detected in body
- `test_waf_block_body_no_false_positive` — normal JSON not flagged
- `test_waf_reason_from_body` — Incapsula pattern produces reason string

### replay.rs tests

- `test_rewrite_url` — scheme/host rewrite preserves path and query
- `test_rewrite_url_with_port` — port is correctly rewritten
- `test_apply_mutations_add` — new header added
- `test_apply_mutations_remove` — header removed via empty value
- `test_apply_mutations_strip_cookies` — cookie header stripped

## Integration tests

Integration tests use `wiremock` to spin up local HTTP servers and test the full pipeline.

```bash
cargo test --test integration
```

### Test modules

**`har_parsing`** — HAR file parsing from fixtures:
- `parse_simple_har` — parses 3-entry HAR
- `har_to_capture_preserves_requests` — method, URL, body, expected_status preserved
- `har_headers_converted` — headers converted to tuples

**`capture_format`** — Capture serialization:
- `load_capture_file` — loads fixture, validates fields
- `capture_round_trip` — serialize then deserialize produces same data

**`replay_engine`** — HTTP replay against wiremock:
- `replay_against_mock_server` — 2 requests, correct status/match
- `replay_captures_body` — response body is stored
- `replay_detects_status_mismatch` — 403 vs expected 200
- `replay_session_round_trip` — save to file, reload, compare
- `replay_no_body_mode` — `capture_body: false` stores null body but non-zero size
- `replay_concurrent_preserves_order` — 3 requests at concurrency 3, results in order

**`diff_engine`** — End-to-end diff:
- `diff_detects_status_difference` — 200 on A, 403 on B
- `diff_detects_body_difference` — same status, different body
- `diff_identical_is_clean` — same server = zero diffs

**`new_features`** — Extended feature tests:
- `replay_computes_body_hash` — SHA256 hash present, 64 hex chars
- `replay_hash_differs_when_body_differs` — different bodies = different hashes
- `error_kind_is_populated_on_failure` — connect to closed port populates error_kind
- `session_metadata_is_populated` — ushio_version and capture_source in meta
- `junit_output_is_valid_xml` — JUnit contains expected XML structure
- `fetch_remote_capture_from_mock` — fetches ushio capture JSON from mock endpoint

## Test fixtures

Located in `tests/fixtures/`:

| File | Description |
|------|-------------|
| `simple.har` | 3 entries: GET 200, POST 200 with body, GET 403 (XSS filter) |
| `capture.json` | 2 requests in ushio format: GET health, POST data |

## Writing new tests

### Unit test pattern

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_something() {
        // Arrange
        let input = make_result(0, 200, vec![]);
        // Act
        let result = some_function(&input);
        // Assert
        assert!(result.is_some());
    }
}
```

### Integration test pattern (with wiremock)

```rust
#[tokio::test]
async fn test_something_end_to_end() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/test"))
        .respond_with(ResponseTemplate::new(200).set_body_string("ok"))
        .mount(&mock_server)
        .await;

    let requests = vec![ushio::capture::CapturedRequest {
        method: "GET".to_string(),
        url: "https://example.com/test".to_string(),
        headers: vec![],
        body: None,
        expected_status: Some(200),
    }];

    let config = ushio::replay::ReplayConfig::default();
    let session = ushio::replay::replay(&requests, &mock_server.uri(), config)
        .await
        .unwrap();

    assert_eq!(session.results[0].status, 200);
}
```
