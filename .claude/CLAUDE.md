# Ushio - Deterministic Edge Traffic Replay

## Purpose
Ushio replays captured HTTP traffic against edge targets to compare behavioral differences. It helps operators understand WAF behavior, compare environments, and debug edge issues.

## Architecture

```
src/
├── main.rs       # CLI entry point (clap with subcommands)
├── har.rs        # HAR 1.2 parsing
├── capture.rs    # Internal capture format
├── replay.rs     # Replay engine
└── diff.rs       # Behavioral diff analysis
```

## Subcommands

- `replay`: Replay captured traffic against target(s)
- `diff`: Compare two replay result files
- `convert`: Convert HAR to ushio capture format

## Design Principles

1. **Deterministic**: Same input, same order, reproducible results
2. **Behavioral focus**: Compare outcomes, not timing
3. **WAF-aware**: Detect blocking decisions and policy differences
4. **Safe**: Read-only observation, never modifies state

## Key Dependencies

- `reqwest`: HTTP client with cookie support
- `serde_json`: HAR and capture parsing
- `similar`: Text diffing for response comparison
- `clap`: CLI with subcommands
- `colored` + `tabled`: Pretty output

## Capture Format

```json
{
  "version": "1.0",
  "source": "browser-export.har",
  "requests": [
    {
      "method": "GET",
      "url": "https://example.com/api/users",
      "headers": [["Authorization", "Bearer ..."]],
      "body": null,
      "expected_status": 200
    }
  ]
}
```

## Diff Detection

WAF differences are detected by:
- Status code patterns (403, 429, etc.)
- WAF-specific headers (X-WAF-*, CF-*, etc.)
- Response body patterns (block pages)

## Testing Strategy

- Unit tests for HAR parsing and diff logic
- Integration tests with wiremock
- Sample HAR files in tests/fixtures/
