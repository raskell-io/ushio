# Capture Format

Ushio uses a JSON-based capture format to represent HTTP traffic for replay.

## Specification

```json
{
  "version": "1.0",
  "source": "browser-export.har",
  "requests": [
    {
      "method": "GET",
      "url": "https://example.com/api/users",
      "headers": [
        ["Authorization", "Bearer eyJ..."],
        ["Accept", "application/json"]
      ],
      "body": null,
      "expected_status": 200
    },
    {
      "method": "POST",
      "url": "https://example.com/api/login",
      "headers": [
        ["Content-Type", "application/json"]
      ],
      "body": "{\"username\":\"test\",\"password\":\"pass\"}",
      "expected_status": 200
    }
  ]
}
```

## Fields

### Root

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `version` | string | yes | Format version. Currently `"1.0"`. Validated on load. |
| `source` | string | no | Origin of the capture (filename, `"stdin"`, `"proxy:..."`, `"remote:..."`) |
| `requests` | array | yes | Ordered list of requests to replay |

### Request

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `method` | string | yes | HTTP method (`GET`, `POST`, `PUT`, `DELETE`, etc.) |
| `url` | string | yes | Full URL including scheme, host, path, and query string |
| `headers` | array of `[name, value]` | yes | Request headers as name/value tuples |
| `body` | string or null | yes | Request body (null for bodyless requests) |
| `expected_status` | integer or null | yes | Expected HTTP status code for mismatch detection (null to skip) |

## URL rewriting

During replay, ushio rewrites the scheme, host, and port of each URL to match the target, preserving path and query string:

```
Original:  https://prod.example.com:443/api/users?q=test
Target:    https://staging.example.com:8443
Replayed:  https://staging.example.com:8443/api/users?q=test
```

## Creating captures

There are three ways to produce a capture file:

### From HAR

```bash
ushio convert session.har -o capture.json
```

### From the capture proxy

```bash
ushio capture --listen 0.0.0.0:8080 --target https://api.example.com -o capture.json
```

### By hand

Write JSON directly. Useful for crafting specific test cases:

```bash
cat > test.json << 'EOF'
{
  "version": "1.0",
  "requests": [
    {
      "method": "GET",
      "url": "https://example.com/api/health",
      "headers": [],
      "body": null,
      "expected_status": 200
    }
  ]
}
EOF
ushio replay test.json -t https://staging.example.com
```

## Replay session format

When you save replay results with `-o`, ushio writes a session file:

```json
{
  "target": "https://staging.example.com",
  "timestamp": "2026-04-22T10:30:00Z",
  "meta": {
    "ushio_version": "0.1.0",
    "capture_source": "capture.json",
    "timeout_secs": 30,
    "concurrency": 1,
    "insecure": false
  },
  "total_requests": 2,
  "successful": 2,
  "failed": 0,
  "status_mismatches": 0,
  "results": [
    {
      "request_index": 0,
      "method": "GET",
      "url": "https://staging.example.com/api/users",
      "status": 200,
      "headers": [["content-type", "application/json"]],
      "body": "{\"users\":[]}",
      "body_hash": "a1b2c3...",
      "body_size": 12,
      "duration_ms": 45,
      "expected_status": 200,
      "status_match": true,
      "error": null,
      "error_kind": null
    }
  ]
}
```

### ReplayResult fields

| Field | Type | Description |
|-------|------|-------------|
| `request_index` | integer | Position in the original capture |
| `method` | string | HTTP method |
| `url` | string | Rewritten URL (target host) |
| `status` | integer | Response status code (0 if request failed) |
| `headers` | array | Response headers |
| `body` | string or null | Response body text (null if binary, too large, or `--no-body`) |
| `body_hash` | string or null | SHA256 hex digest of the response body |
| `body_size` | integer | Response body size in bytes |
| `duration_ms` | integer | Request duration in milliseconds |
| `expected_status` | integer or null | Expected status from the capture |
| `status_match` | boolean | Whether status matched expected |
| `error` | string or null | Error message if request failed |
| `error_kind` | string or null | Error category: `timeout`, `dns`, `connect`, `tls`, `request`, `response`, `unknown` |
