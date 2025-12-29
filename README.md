# Ushio

**Deterministic edge traffic replay**

Ushio (潮, "tide") is a traffic replay tool to understand edge and WAF behavior. Like the tide flowing in and out, Ushio replays captured traffic deterministically to compare behavior across environments.

## Problem

When something breaks, teams ask:
- Why was this blocked?
- Would it still block after a change?
- Why does staging differ from prod?

Answers today require log spelunking or risky live testing.

## Solution

Ushio replays captured HTTP traffic in a controlled way against one or more edge targets and compares *behavior*, not performance.

## Features

- **HAR Support**: Replay from HAR files or structured captures
- **Deterministic Ordering**: Same order, every time
- **Header Mutation**: Modify headers/cookies for testing
- **Multi-Target Comparison**: Prod vs staging, before vs after
- **Behavioral Diff**: Status codes, headers, WAF decisions

## Installation

```bash
cargo install ushio
```

## Usage

```bash
# Replay a HAR file against a target
ushio replay capture.har --target https://staging.example.com

# Compare behavior between two environments
ushio replay capture.har \
  --target https://prod.example.com \
  --target https://staging.example.com

# Diff two replay results
ushio diff prod-results.json staging-results.json

# Convert HAR to ushio format
ushio convert browser-capture.har -o capture.json
```

## Example Workflow

```bash
# 1. Capture traffic (using browser dev tools, export as HAR)

# 2. Replay against prod and staging
ushio replay traffic.har --target https://prod.example.com > prod.json
ushio replay traffic.har --target https://staging.example.com > staging.json

# 3. Diff the results
ushio diff prod.json staging.json --only-diff
```

## Example Output

```
ushio - behavioral diff

Comparing: prod.example.com vs staging.example.com
Requests: 42 total, 38 identical, 4 different

Differences:
  [12] POST /api/login
       Status: 200 vs 403 (WAF block on staging)

  [23] GET /api/users?filter=<script>
       Status: 403 vs 200 (WAF miss on staging)
```

## Philosophy

- **Reproducible truth** for edge security behavior
- **Bridges operators, security, and developers**
- **Eliminates guesswork** and politics
- **Safe to run** against production
- **Foundation for AI-assisted** WAF analysis

## Part of the Raskell.io Family

Ushio is part of the [raskell.io](https://raskell.io) ecosystem, alongside:
- [Sentinel](https://sentinel.raskell.io) - Security-first reverse proxy
- [Sango](https://github.com/raskell-io/sango) - Edge diagnostics
- [Tanuki](https://tanuki.raskell.io) - Agent registry

## License

MIT OR Apache-2.0
