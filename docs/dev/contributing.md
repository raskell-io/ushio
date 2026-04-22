# Contributing

## Prerequisites

- Rust stable (1.75+)
- Git

## Building

```bash
git clone https://github.com/raskell-io/ushio.git
cd ushio
cargo build
```

Release build:

```bash
cargo build --release
```

## Running tests

```bash
# All tests
cargo test

# Unit tests only (fast, no network)
cargo test --lib

# Integration tests only (uses wiremock)
cargo test --test integration

# A specific test
cargo test test_body_diff_different
```

## Code quality checks

CI runs all three — make sure they pass locally before pushing:

```bash
# Formatting
cargo fmt -- --check

# Linting (warnings = errors in CI)
cargo clippy -- -D warnings

# Tests
cargo test
```

## Project structure

```
.
├── src/
│   ├── lib.rs          # Library root
│   ├── main.rs         # Binary entry point
│   ├── har.rs          # HAR parsing
│   ├── capture.rs      # Capture format
│   ├── replay.rs       # Replay engine
│   ├── diff.rs         # Diff engine
│   ├── output.rs       # Output formatters
│   └── proxy.rs        # Capture proxy + remote fetch
├── tests/
│   ├── integration.rs  # Integration tests (wiremock-based)
│   └── fixtures/       # Sample HAR and capture files
├── docs/
│   ├── *.md            # User-facing documentation
│   └── dev/            # Developer documentation
└── .claude/
    ├── CLAUDE.md       # AI assistant context
    └── ROADMAP.md      # Feature status
```

## Adding a new feature

1. **Implement** in the appropriate module (`replay.rs`, `diff.rs`, etc.)
2. **Expose** through `lib.rs` if it's public API
3. **Wire CLI** in `main.rs` if it has a flag
4. **Add unit tests** in the module's `#[cfg(test)]` section
5. **Add integration tests** in `tests/integration.rs` if it's end-to-end
6. **Update docs** in `docs/` for user-facing changes

## Adding a new WAF pattern

WAF body patterns are in `diff.rs` in the `WAF_BODY_PATTERNS` constant. Add the pattern as a lowercase string — matching is case-insensitive.

```rust
const WAF_BODY_PATTERNS: &[&str] = &[
    // ... existing patterns ...
    "your new pattern here",
];
```

Add a test in the `tests` module:

```rust
#[test]
fn test_waf_block_body_new_vendor() {
    let result = make_result_with_body(0, 200, vec![], Some("Your New Pattern Here"));
    assert!(is_waf_block(&result));
}
```

## Adding a new output format

1. Add a variant to `OutputFormat` in `main.rs`
2. Implement `print_replay_<format>()` and `print_diff_<format>()` in `output.rs`
3. Wire it into the `match args.format` blocks in `main.rs`

## Test fixtures

Test fixtures live in `tests/fixtures/`:

- `simple.har` — 3-request HAR with GET, POST, and a 403
- `capture.json` — 2-request ushio capture

To add a new fixture, create the file and reference it in tests via:

```rust
fn fixture_path(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join(name)
}
```

Note: `.gitignore` has `*.har` but an exception for `tests/fixtures/*.har`.
