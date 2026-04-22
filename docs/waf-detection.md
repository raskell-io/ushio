# WAF Detection

Ushio detects WAF (Web Application Firewall) blocking decisions through three signals: status codes, headers, and response body patterns.

## Status codes

These status codes are treated as WAF blocks:

| Status | Meaning |
|--------|---------|
| 403 | Forbidden (most common WAF block) |
| 429 | Too Many Requests (rate limiting) |
| 503 | Service Unavailable (often used by WAFs under load or during challenges) |

## Headers

Ushio checks for the presence of headers with these prefixes:

- `x-waf-*` (generic WAF headers)
- `x-blocked*` (block indicators)

If any of these headers are present, the response is considered a WAF block regardless of status code.

## Response body patterns

When response bodies are captured, ushio scans them (case-insensitive) for known WAF block page signatures:

### Generic patterns
- `access denied`
- `request blocked`
- `forbidden by security policy`

### Cloudflare
- `/cdn-cgi/challenge-platform/`
- `attention required! | cloudflare`
- `ray id:`
- `cloudflare to restrict access`

### Akamai
- `reference&#32;&#35;` (HTML-encoded reference ID)
- `access denied | akamai`
- `akamaighost`

### AWS WAF
- `request blocked by aws waf`

### Imperva / Incapsula
- `incapsula incident id`
- `powered by incapsula`

### ModSecurity
- `mod_security`
- `modsecurity`

### F5 BIG-IP
- `the requested url was rejected`
- `support id:`

### Sucuri
- `sucuri website firewall`

### Barracuda
- `barracuda networks`

## How it's used

### In replay output

When a response is detected as a WAF block, the pretty output flags it:

```
    #12 POST /api/login
      Expected: 200, Got: 403
```

### In diff output

When two environments differ in WAF behavior, the diff shows:

```
    #12 POST /api/login
      Status: 200 -> 403
      WAF: allowed -> blocked
        Right: x-waf-rule: 942100
```

The WAF reason is extracted from (in priority order):
1. `x-waf-rule` header
2. `x-waf-action` header
3. `x-blocked-by` header
4. `x-blocked` header
5. Status code (e.g. `HTTP 403`)
6. Body pattern match (e.g. `body match: powered by incapsula`)

### Body-based detection catches stealth blocks

Some WAFs return HTTP 200 with a challenge page instead of a clean 403. Without body inspection, these look like successful responses. Ushio's body pattern matching catches these:

```
    #7 GET /api/data
      Status: 200 (both)
      WAF: allowed -> blocked
        Right: body match: attention required! | cloudflare
```

## Diff header comparison

Beyond WAF-specific headers, the diff engine also compares these headers between environments:

| Header | Why |
|--------|-----|
| `cf-ray` | Cloudflare request ID |
| `cf-cache-status` | Cloudflare cache behavior |
| `x-cache`, `x-cache-status` | CDN cache behavior |
| `server` | Origin server identity |
| `x-frame-options` | Clickjacking protection |
| `content-security-policy` | CSP policy |
| `strict-transport-security` | HSTS configuration |
| `x-content-type-options` | MIME sniffing protection |
