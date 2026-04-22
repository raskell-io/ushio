# Getting Started

## Installation

### From source

```bash
git clone https://github.com/raskell-io/ushio.git
cd ushio
cargo build --release
```

The binary is at `target/release/ushio`.

### With cargo

```bash
cargo install ushio
```

### Shell completions

Generate completions for your shell and source them:

```bash
# Bash
ushio completions bash > ~/.local/share/bash-completion/completions/ushio

# Zsh
ushio completions zsh > ~/.zfunc/_ushio

# Fish
ushio completions fish > ~/.config/fish/completions/ushio.fish
```

## Quick start

### 1. Get a HAR file

Export a HAR from your browser's DevTools (Network tab > Export HAR), or use the built-in capture proxy:

```bash
# Option A: Convert a browser HAR export
ushio convert session.har -o capture.json

# Option B: Record live traffic through the capture proxy
ushio capture --listen 0.0.0.0:8080 --target https://api.example.com -o capture.json
# Point your client at localhost:8080, then Ctrl-C to save
```

### 2. Replay against a target

```bash
ushio replay capture.json -t https://staging.example.com
```

### 3. Compare two environments

```bash
# Replay against both
ushio replay capture.json -t https://staging.example.com -o staging.json
ushio replay capture.json -t https://prod.example.com -o prod.json

# Diff the results
ushio diff staging.json prod.json
```

The diff exits with code 1 if differences are found, making it usable in CI.

## What's next

- [CLI Reference](./cli-reference.md) for the full list of flags and options
- [Workflows](./workflows.md) for common use cases
- [WAF Detection](./waf-detection.md) for how ushio identifies blocking behavior
- [Capture Format](./capture-format.md) for the ushio JSON format spec
