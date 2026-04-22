<div align="center">

<h1 align="center">
  <img src=".github/static/ushio-icon.png" alt="ushio icon" width="96" />
  <br>
  Ushio
</h1>

<p align="center">
  <em>Deterministic edge traffic replay.</em><br>
  <em>Know why it blocked before it breaks.</em>
</p>

<p align="center">
  <a href="https://www.rust-lang.org/">
    <img alt="Rust" src="https://img.shields.io/badge/Rust-stable-000000?logo=rust&logoColor=white&style=for-the-badge">
  </a>
  <a href="https://github.com/raskell-io/ushio">
    <img alt="CLI" src="https://img.shields.io/badge/CLI-replay-f5a97f?style=for-the-badge">
  </a>
  <a href="LICENSE">
    <img alt="License" src="https://img.shields.io/badge/License-MIT%20%7C%20Apache--2.0-c6a0f6?style=for-the-badge">
  </a>
</p>

<p align="center">
  <a href="docs/getting-started.md">Getting Started</a> •
  <a href="docs/cli-reference.md">CLI Reference</a> •
  <a href="docs/workflows.md">Workflows</a> •
  <a href="docs/dev/contributing.md">Contributing</a>
</p>

<hr />

</div>

Ushio (潮, "tide") replays captured HTTP traffic against edge targets to compare behavioral differences. It helps operators understand WAF behavior, compare environments, and debug edge issues.

It answers the questions operators dread:
- *Why was this blocked?*
- *Would it still block after a change?*
- *Why does staging differ from prod?*

---

## Quick start

```bash
cargo install ushio
```

```bash
# Convert a browser HAR export
ushio convert session.har -o capture.json

# Replay against staging
ushio replay capture.json -t https://staging.example.com

# Compare staging vs prod
ushio replay capture.json -t https://staging.example.com -o staging.json
ushio replay capture.json -t https://prod.example.com -o prod.json
ushio diff staging.json prod.json
```

---

## Features

| Feature | Description |
|---------|-------------|
| **HAR + capture formats** | Replay from browser HAR exports or ushio's JSON format |
| **URL rewriting** | Replay prod traffic against staging transparently |
| **Header mutation** | Add, replace, or remove headers per request |
| **WAF detection** | Identify blocks via status codes, headers, and body patterns |
| **Body diff** | Unified text diff of response bodies with SHA256 hashing |
| **Behavioral diff** | Compare status, headers, body, and WAF decisions across targets |
| **Concurrent replay** | Ordered results with configurable in-flight concurrency |
| **Rate limiting** | Per-request delay for safe production replay |
| **Capture proxy** | Built-in reverse proxy that records traffic |
| **Remote fetch** | Pull request logs from Sentinel or compatible endpoints |
| **CI integration** | JUnit XML output, assertion mode with exit codes |
| **Proxy support** | Route through HTTP or SOCKS proxies |
| **Shell completions** | Bash, Zsh, Fish, Elvish, PowerShell |

---

## Example output

### Replay
```
ushio traffic replay
────────────────────────────────────────────────────────────

  Target: https://staging.example.com
  Time: 2025-01-15 10:30:00 UTC

  Requests: 42
  Successful: 40
  Mismatches: 2

  Issues

    #12 POST /api/login
      Expected: 200, Got: 403

    #23 GET /api/users?filter=<script>
      Expected: 200, Got: 403

────────────────────────────────────────────────────────────
```

### Diff
```
ushio diff results
────────────────────────────────────────────────────────────

  Left: https://prod.example.com
  Right: https://staging.example.com

  Total: 42
  Identical: 40
  Different: 2
  WAF diffs: 2

  Differences

    #12 POST /api/login
      Status: 200 → 403
      WAF: allowed → blocked
        Right: HTTP 403

    #23 GET /api/users?filter=<script>
      Status: 403 → 200
      WAF: blocked → allowed
        Left: x-waf-rule: 942100

────────────────────────────────────────────────────────────
```

---

## Documentation

### User guides
- [Getting Started](docs/getting-started.md) — Installation, first steps
- [CLI Reference](docs/cli-reference.md) — All commands, flags, and exit codes
- [Workflows](docs/workflows.md) — Debugging WAF blocks, CI integration, recording traffic
- [Capture Format](docs/capture-format.md) — JSON format specification
- [WAF Detection](docs/waf-detection.md) — How blocking is detected, supported vendors

### Developer guides
- [Architecture](docs/dev/architecture.md) — Module overview, data flow, design decisions
- [Contributing](docs/dev/contributing.md) — Building, testing, adding features
- [Testing](docs/dev/testing.md) — Test strategy, fixtures, writing tests

---

## Exit codes

| Code | Context | Meaning |
|------|---------|---------|
| 0 | `replay` | All requests completed, no mismatches (or `--assert-no-mismatch` not set) |
| 0 | `diff` | No differences found |
| 1 | `diff` | Differences detected |
| 2 | `replay` | Status mismatches detected (with `--assert-no-mismatch`) |

---

## Part of the Raskell.io family

Ushio is part of the [raskell.io](https://raskell.io) ecosystem:
- [Sentinel](https://sentinel.raskell.io) — Security-first reverse proxy built on Pingora
- [Sango](https://github.com/raskell-io/sango) — Operator-grade edge diagnostics
- [Tanuki](https://tanuki.raskell.io) — Agent registry

---

## License

MIT OR Apache-2.0
