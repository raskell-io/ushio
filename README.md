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
  <a href="#installation">Installation</a> •
  <a href="#usage">Usage</a> •
  <a href="#features">Features</a> •
  <a href="#philosophy">Philosophy</a>
</p>

<hr />

</div>

Ushio (潮, "tide") is a traffic replay tool to understand edge and WAF behavior. Like the tide flowing in and out, Ushio replays captured traffic deterministically to compare behavior across environments.

It answers the questions operators dread:
- *Why was this blocked?*
- *Would it still block after a change?*
- *Why does staging differ from prod?*

---

## Features

| Feature | Description |
|---------|-------------|
| **HAR Support** | Replay from browser HAR exports or ushio capture format |
| **URL Rewriting** | Replay prod traffic against staging transparently |
| **Header Mutation** | Add, replace, or remove headers for testing |
| **Cookie Stripping** | Test without session state |
| **WAF Detection** | Identify blocks via status codes and WAF headers |
| **Behavioral Diff** | Compare status, headers, and WAF decisions across targets |

---

## Installation

```bash
cargo install ushio
```

Or build from source:

```bash
git clone https://github.com/raskell-io/ushio.git
cd ushio
cargo build --release
```

---

## Usage

```bash
# Convert HAR to ushio format
ushio convert session.har -o capture.json

# Replay against a target
ushio replay capture.json -t https://staging.example.com

# Replay with header mutation
ushio replay capture.json -t https://staging.example.com \
  --header "Authorization:Bearer test-token"

# Save results to file
ushio replay capture.json -t https://staging.example.com -o staging.json

# Compare two replay results
ushio diff staging.json prod.json

# Show only differences
ushio diff staging.json prod.json --only-diff
```

---

## Example Output

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

## Example Workflow

### Debugging a WAF block
```bash
# 1. Export HAR from browser (when request worked)
# 2. Convert to ushio format
ushio convert working-session.har -o capture.json

# 3. Replay against staging (where it's blocked)
ushio replay capture.json -t https://staging.example.com
```

### Comparing environments
```bash
# Replay same capture against two environments
ushio replay capture.json -t https://staging.example.com -o staging.json
ushio replay capture.json -t https://prod.example.com -o prod.json

# Diff the results
ushio diff staging.json prod.json --only-diff
```

---

## Philosophy

- **Reproducible truth** for edge security behavior
- **Bridges operators, security, and developers**
- **Eliminates guesswork** and politics
- **Safe to run** against production (read-only replay)
- **Foundation for AI-assisted** WAF analysis

---

## Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success (replay complete, or diff found no differences) |
| 1 | Differences found (diff command only) |

---

## Part of the Raskell.io Family

Ushio is part of the [raskell.io](https://raskell.io) ecosystem, alongside:
- [Sentinel](https://sentinel.raskell.io) - Security-first reverse proxy built on Pingora
- [Sango](https://github.com/raskell-io/sango) - Operator-grade edge diagnostics
- [Tanuki](https://tanuki.raskell.io) - Agent registry

---

## License

MIT OR Apache-2.0
