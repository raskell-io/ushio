# Ushio Roadmap

All items below have been implemented.

## Tier 1 — Core (done)

- [x] **Response body capture and diff** — body stored in ReplayResult (256KB cap), unified diff via `similar`, `--no-body` flag
- [x] **Body-based WAF detection** — 25+ patterns (Cloudflare, Akamai, AWS WAF, Imperva, ModSecurity, F5, Sucuri, Barracuda)
- [x] **Integration tests with fixtures** — wiremock-based end-to-end tests, HAR/capture fixtures in `tests/fixtures/`

## Tier 2 — Usability (done)

- [x] **Rate limiting** — `--delay <ms>` between sequential requests
- [x] **Insecure TLS** — `--insecure` for self-signed certs
- [x] **Request filtering** — `--filter`, `--method`, `--range`
- [x] **Concurrent replay** — `--concurrency <n>` via `futures::stream::buffered()` with preserved ordering
- [x] **Progress indication** — TTY-aware stderr progress line

## Tier 3 — Polish (done)

- [x] **Session metadata** — ReplayMeta with version, capture source, config
- [x] **Stdin support** — `ushio convert - -o capture.json` reads from stdin
- [x] **Capture format version checking** — validates `version` field on load
- [x] **Error categorization** — ErrorKind enum: Timeout, Dns, Connect, Tls, Request, Response, Unknown
- [x] **Dead code cleanup** — zero warnings, clean `clippy -D warnings`

## Tier 4 — Extended features (done)

- [x] **Proxy support** — `--proxy http://... | socks5://...` via reqwest
- [x] **Shell completions** — `ushio completions bash|zsh|fish` via clap_complete
- [x] **Assertion mode for CI** — `--assert-no-mismatch` exits code 2 on status mismatches
- [x] **JUnit output** — `-f junit` for replay and diff, plugs into CI dashboards
- [x] **Response body hashing** — SHA256 in `body_hash` field, enables hash-only comparison when body not captured
- [x] **HAR recording proxy** — `ushio capture --listen 0.0.0.0:8080 --target https://...` reverse proxy that records traffic
- [x] **Remote capture** — `ushio capture --from-url https://sentinel/api/logs` fetches from Sentinel or compatible endpoint

## Out of scope

- AI-assisted WAF analysis (future product)
- mTLS / client certificates
- Redirect following option
