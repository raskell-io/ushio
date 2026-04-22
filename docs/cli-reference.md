# CLI Reference

## Global options

| Flag | Description |
|------|-------------|
| `-f, --format <FORMAT>` | Output format: `pretty` (default), `json`, `compact`, `junit` |
| `-v, --verbose` | Enable debug-level logging |
| `-h, --help` | Print help |
| `-V, --version` | Print version |

---

## `ushio replay`

Replay captured traffic against one or more targets.

```
ushio replay [OPTIONS] --target <TARGET> <CAPTURE>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<CAPTURE>` | Path to a HAR file or ushio capture file |

### Options

| Flag | Description | Default |
|------|-------------|---------|
| `-t, --target <URL>` | Target URL to replay against (repeatable for multiple targets) | required |
| `-o, --output <FILE>` | Save replay session to file | stdout |
| `--timeout <SECS>` | Per-request timeout | `30` |
| `--concurrency <N>` | Number of concurrent in-flight requests. Results remain in order. | `1` |
| `--delay <MS>` | Delay between requests in milliseconds (sequential mode only) | `0` |
| `--header <NAME:VALUE>` | Add/replace a header. Use `Name:` (empty value) to remove. Repeatable. | |
| `--strip-cookies` | Remove all `Cookie` headers from requests | `false` |
| `--no-body` | Don't capture response bodies (saves memory; hashes still computed) | `false` |
| `--insecure` | Accept invalid TLS certificates | `false` |
| `--proxy <URL>` | Route through an HTTP or SOCKS proxy (`http://`, `socks5://`) | |
| `--filter <PATTERN>` | Only replay requests whose URL contains this substring | |
| `--method <METHODS>` | Comma-separated list of HTTP methods to include (e.g. `GET,POST`) | |
| `--range <RANGE>` | Index range to replay: `5-10`, `5-`, `-10`, or `5` | |
| `--assert-no-mismatch` | Exit with code 2 if any status mismatches are found | `false` |

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | Replay completed, all statuses matched (or no expected status set) |
| 2 | Status mismatches detected (only with `--assert-no-mismatch`) |

### Examples

```bash
# Basic replay
ushio replay capture.json -t https://staging.example.com

# Replay with auth header, save results
ushio replay capture.json -t https://staging.example.com \
  --header "Authorization:Bearer tok_123" \
  -o staging.json

# Fast concurrent replay through a proxy
ushio replay capture.json -t https://staging.example.com \
  --concurrency 10 --proxy http://localhost:8080

# CI mode: fail if anything mismatches, output JUnit
ushio replay capture.json -t https://staging.example.com \
  --assert-no-mismatch -f junit > results.xml

# Only replay POST requests to /api/
ushio replay capture.json -t https://staging.example.com \
  --method POST --filter /api/

# Rate-limited replay against production
ushio replay capture.json -t https://prod.example.com --delay 100
```

---

## `ushio diff`

Compare two replay session files and report behavioral differences.

```
ushio diff [OPTIONS] <LEFT> <RIGHT>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<LEFT>` | First replay session file |
| `<RIGHT>` | Second replay session file |

### Options

| Flag | Description |
|------|-------------|
| `--only-diff` | Only print requests that differ |

### Exit codes

| Code | Meaning |
|------|---------|
| 0 | No differences found |
| 1 | Differences detected |

### What gets compared

- **Status codes** — any difference is flagged
- **Response bodies** — unified diff via the `similar` crate (hash-based fast path when bodies not captured)
- **WAF-relevant headers** — `x-waf-*`, `x-blocked*`, `cf-ray`, `x-cache`, `server`, security headers
- **WAF decisions** — blocked vs. allowed based on status codes, headers, and body patterns

### Examples

```bash
# Pretty diff
ushio diff staging.json prod.json

# Only show differences, compact
ushio diff staging.json prod.json --only-diff -f compact

# JUnit for CI
ushio diff staging.json prod.json -f junit > diff-report.xml
```

---

## `ushio convert`

Convert a HAR 1.2 file to ushio capture format.

```
ushio convert [OPTIONS] <INPUT>
```

### Arguments

| Argument | Description |
|----------|-------------|
| `<INPUT>` | Path to HAR file, or `-` to read from stdin |

### Options

| Flag | Description |
|------|-------------|
| `-o, --output <FILE>` | Output file (default: stdout) |

### Examples

```bash
# File to file
ushio convert session.har -o capture.json

# Pipe from stdin
cat session.har | ushio convert - -o capture.json

# Pipe to stdout
ushio convert session.har | jq '.requests | length'
```

---

## `ushio capture`

Capture traffic via a reverse proxy or by fetching logs from a remote endpoint.

```
ushio capture [OPTIONS]
```

Requires either `--listen` + `--target` (proxy mode) or `--from-url` (fetch mode).

### Options

| Flag | Description |
|------|-------------|
| `--listen <ADDR>` | Listen address for proxy mode (e.g. `0.0.0.0:8080`) |
| `--target <URL>` | Target URL to forward requests to |
| `--from-url <URL>` | Fetch request logs from a remote endpoint |
| `-o, --output <FILE>` | Output file (default: `capture.json` in proxy mode, stdout in fetch mode) |
| `--insecure` | Accept invalid TLS certificates on the target |

### Proxy mode

Starts a reverse proxy that forwards all requests to the target and records them. Press Ctrl-C to stop and save.

```bash
ushio capture --listen 0.0.0.0:8080 --target https://api.example.com -o capture.json
# Point your client at http://localhost:8080
# Ctrl-C to save
```

### Fetch mode

Fetches request logs from a remote URL. Accepts three JSON formats:

1. **Ushio capture format**: `{ "version": "1.0", "requests": [...] }`
2. **Plain array**: `[{ "method": "GET", "url": "...", ... }]`
3. **Entries wrapper** (Sentinel-compatible): `{ "entries": [{ "method": "GET", "url": "...", ... }] }`

```bash
# Fetch from a Sentinel instance
ushio capture --from-url https://sentinel.internal/api/traffic-logs -o capture.json

# Fetch with self-signed cert
ushio capture --from-url https://staging-sentinel/api/logs --insecure
```

---

## `ushio completions`

Generate shell completion scripts.

```
ushio completions <SHELL>
```

### Arguments

| Argument | Values |
|----------|--------|
| `<SHELL>` | `bash`, `zsh`, `fish`, `elvish`, `powershell` |

### Examples

```bash
# Generate and install for zsh
ushio completions zsh > ~/.zfunc/_ushio

# One-liner for bash
source <(ushio completions bash)
```
