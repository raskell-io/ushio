# Workflows

## Debugging a WAF block

A request works from your laptop but gets blocked on staging. What changed?

```bash
# 1. Export HAR from your browser (Network tab > Export HAR)
#    while the request still works

# 2. Convert to ushio format
ushio convert working-session.har -o capture.json

# 3. Replay against staging
ushio replay capture.json -t https://staging.example.com

# 4. Look at the output — mismatches show expected vs actual status,
#    and WAF detection tells you if a block page was returned
```

If you need to narrow it down:

```bash
# Only replay the specific endpoint
ushio replay capture.json -t https://staging.example.com \
  --filter /api/login --method POST
```

## Comparing two environments

Same traffic, two environments — where do they differ?

```bash
# Replay against both, save results
ushio replay capture.json -t https://staging.example.com -o staging.json
ushio replay capture.json -t https://prod.example.com -o prod.json

# Diff
ushio diff staging.json prod.json --only-diff
```

The diff reports:
- Status code changes (e.g. 200 on prod, 403 on staging)
- WAF decision changes (blocked vs allowed, with the reason)
- Response body differences (unified diff)
- Security header changes

## CI pipeline integration

### Assert on replay

Use `--assert-no-mismatch` to fail the build when replay results don't match expectations:

```yaml
# GitHub Actions example
- name: Replay traffic against staging
  run: |
    ushio replay tests/fixtures/capture.json \
      -t ${{ env.STAGING_URL }} \
      --assert-no-mismatch \
      -f junit > replay-results.xml

- name: Upload test results
  uses: actions/upload-artifact@v4
  if: always()
  with:
    name: replay-results
    path: replay-results.xml
```

### Diff in CI

Use the diff exit code (1 = differences found) as a gate:

```yaml
- name: Compare staging vs prod
  run: |
    ushio replay capture.json -t $STAGING_URL -o staging.json
    ushio replay capture.json -t $PROD_URL -o prod.json
    ushio diff staging.json prod.json -f junit > diff-report.xml
```

### JUnit output

Both `replay` and `diff` support `-f junit` which produces standard JUnit XML. This integrates with:
- GitHub Actions (via test report actions)
- Jenkins (JUnit plugin)
- GitLab CI (JUnit artifact reports)
- Any CI system that understands JUnit XML

## Recording live traffic

Instead of exporting HAR from a browser, use the capture proxy to record traffic from any HTTP client:

```bash
# Start the capture proxy
ushio capture --listen 0.0.0.0:8080 --target https://api.example.com -o capture.json

# In another terminal, run your test suite / curl commands / whatever
curl http://localhost:8080/api/users
curl -X POST http://localhost:8080/api/login -d '{"user":"test"}'

# Ctrl-C in the proxy terminal to save
```

The proxy records every request/response pair with the actual status code as the expected status.

## Fetching from Sentinel

If you run [Sentinel](https://sentinel.raskell.io) as your edge proxy, you can pull captured traffic directly:

```bash
# Fetch recent traffic logs
ushio capture --from-url https://sentinel.internal/api/traffic-logs -o capture.json

# Then replay against a different environment
ushio replay capture.json -t https://staging.example.com
```

The fetch endpoint accepts three JSON formats:
- Ushio capture format (`{ "version": "1.0", "requests": [...] }`)
- Plain request array (`[{ "method": "GET", ... }]`)
- Entries wrapper (`{ "entries": [...] }`)

## Working through a proxy

If your target is only reachable through a corporate proxy:

```bash
# HTTP proxy
ushio replay capture.json -t https://internal.example.com \
  --proxy http://proxy.corp:8080

# SOCKS5 proxy
ushio replay capture.json -t https://internal.example.com \
  --proxy socks5://proxy.corp:1080
```

## Large capture files

For captures with hundreds or thousands of requests:

```bash
# Concurrent replay (10 in-flight, results still ordered)
ushio replay large-capture.json -t https://staging.example.com \
  --concurrency 10

# Skip body capture to save memory
ushio replay large-capture.json -t https://staging.example.com \
  --no-body

# Replay a subset
ushio replay large-capture.json -t https://staging.example.com \
  --range 0-49
```

Even with `--no-body`, SHA256 hashes are still computed for every response, so `diff` can detect body changes via hash comparison.
