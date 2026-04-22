# Ushio Roadmap

## Tier 1 — Core gaps (tool can't fulfill its stated purpose without these)

### 1. Response body capture and diff
The replay engine reads the response body but discards it, keeping only `body_size`. The `similar` crate is in deps but unused. Without body content, the diff engine can't compare what two environments actually returned. WAFs frequently return 200 with a challenge page rather than a clean 403 — the most insidious differences are invisible today.

**Needed:** Store response body (or hash + truncated excerpt) in `ReplayResult`. Add body comparison to `diff_results()` using `similar`. Add `--body` / `--no-body` flag to control storage.

### 2. Body-based WAF detection
CLAUDE.md lists "Response body patterns (block pages)" as a detection method, but `is_waf_block()` only checks status codes and headers. Misses Cloudflare challenge pages, Akamai block pages, generic "Access Denied" patterns, and custom block page signatures.

**Needed:** Pattern list in `is_waf_block()` that scans body content for known WAF signatures. Depends on #1.

### 3. Integration tests with fixtures
`wiremock` is in dev-deps but unused. `tests/fixtures/` is documented but doesn't exist. The 9 existing tests only cover unit-level logic. Nothing tests the replay → diff pipeline end-to-end.

**Needed:** Sample HAR files in `tests/fixtures/`, wiremock-based end-to-end tests, round-trip capture serialization tests, HAR parsing edge cases.

## Tier 2 — Practical usability (impractical for real use without these)

### 4. Rate limiting / delay
No throttle mechanism. Replaying hundreds of requests against production with zero delay will trip rate limits. Essential for the "safe to run against production" promise.

**Needed:** `--delay <ms>` between requests and/or `--rps <n>` flag.

### 5. Insecure TLS option
No way to accept self-signed certificates. Staging environments almost always use self-signed or internal CA certs.

**Needed:** `--insecure` flag that sets `danger_accept_invalid_certs(true)` on the reqwest client.

### 6. Request filtering
No way to replay a subset of a capture. Operators debugging one endpoint must replay everything.

**Needed:** `--filter <url-pattern>`, `--method GET,POST`, `--range 10-20`.

### 7. Concurrent replay
`ReplayConfig.concurrency` exists but is hardcoded to 1. `futures` is in deps but unused. Sequential replay is impractical for large captures.

**Needed:** `--concurrency <n>` flag using `futures::stream::buffered()` with ordered results.

### 8. Progress indication
No output until all requests complete. Unusable interactively for large captures.

**Needed:** Stderr progress counter when outputting to a TTY.

## Tier 3 — Polish

### 9. Session metadata
`ReplaySession` doesn't record capture file path, config used, or ushio version. Sessions aren't reproducible as audit artifacts.

### 10. Stdin support for `convert`
No way to pipe: `curl ... | ushio convert - -o capture.json`.

### 11. Capture format version checking
`version: "1.0"` exists but is never validated on load.

### 12. Error categorization
Errors stored as opaque strings with no distinction between timeout, DNS failure, connection refused, TLS error.

### 13. Dead dependency cleanup
`similar`, `futures`, and `tabled` are in `Cargo.toml` but unused. Either wire them in or remove them. The 10 dead-code warnings would fail CI's `clippy -D warnings`.

## Out of scope

- AI-assisted WAF analysis (future)
- HAR export from sessions
- mTLS / client certificates
- Redirect following option
