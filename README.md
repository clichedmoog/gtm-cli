# GTM CLI

[![GitHub stars](https://img.shields.io/github/stars/clichedmoog/gtm-cli)](https://github.com/clichedmoog/gtm-cli/stargazers) [![MIT License](https://img.shields.io/badge/License-MIT-yellow.svg)](LICENSE) [![Rust](https://img.shields.io/badge/Rust-1.70+-orange.svg)](https://www.rust-lang.org/) [![CI](https://github.com/clichedmoog/gtm-cli/actions/workflows/ci.yml/badge.svg)](https://github.com/clichedmoog/gtm-cli/actions/workflows/ci.yml) [![npm](https://img.shields.io/npm/v/gtmcli)](https://www.npmjs.com/package/gtmcli)

A command-line interface for the Google Tag Manager API v2 — built for humans and AI agents.

```
gtm <resource> <action> [flags]
```

## Quick Start

```bash
# Install
npm install -g gtmcli

# Authenticate (opens browser)
gtm auth login

# Set defaults
gtm config setup

# List your tags
gtm tags list

# Create a GA4 tag
gtm tags create --name "GA4 - Page View" --type gaawc \
  --firing-trigger-id 2 \
  --params '{"measurementId":"G-XXXXXXX"}'

# One-command GA4 setup
gtm setup ga4 --measurement-id G-XXXXXXX

# Publish
gtm versions create --name "v1.0" --notes "Initial release"
gtm versions publish --version-id 1
```

## Installation

### npm (recommended)

```bash
npm install -g gtmcli
```

### Quick Install (macOS)

```bash
curl -fsSL https://github.com/clichedmoog/gtm-cli/releases/latest/download/gtm-$(uname -m | sed 's/arm64/aarch64/')-apple-darwin.tar.gz | tar xz -C /usr/local/bin
```

### Quick Install (Linux)

```bash
curl -fsSL https://github.com/clichedmoog/gtm-cli/releases/latest/download/gtm-$(uname -m)-unknown-linux-gnu.tar.gz | tar xz -C /usr/local/bin
```

### From source

```bash
git clone https://github.com/clichedmoog/gtm-cli.git
cd gtmcli
cargo install --path .
```

## Usage Examples

### Tags & Triggers

```bash
# List tags
gtm tags list

# Create a Custom HTML tag
gtm tags create --name "Tracking Pixel" --type html \
  --firing-trigger-id 1 \
  --params '{"html":"<img src=\"https://example.com/pixel\">"}'

# Create a Custom Event trigger
gtm triggers create --name "Button Click" --type customEvent \
  --custom-event-filter "button_click"

# Create a Data Layer variable
gtm variables create --name "User ID" --type v --params '{"name":"userId"}'
```

### Quick Setup Workflows

```bash
gtm setup ga4 --measurement-id G-XXXXXXX
gtm setup facebook-pixel --pixel-id 1234567890
gtm setup form-tracking --measurement-id G-XXXXXXX
```

### Output Formats

```bash
# Table (default in terminal)
gtm tags list

# JSON (default when piped)
gtm tags list | jq '.[].name'

# Compact (ID + name only)
gtm tags list --format compact
```

### Version Management

```bash
gtm versions create --name "v1.0" --notes "Initial release"
gtm versions publish --version-id 1
gtm versions live                  # Show live version
```

### Export & Import

```bash
gtm workspaces export -o backup.json
gtm workspaces import -i backup.json
```

### Safety

All delete commands require `--force`:

```bash
gtm tags delete --tag-id 42 --force
```

Use `--dry-run` to preview any changes:

```bash
gtm tags create --name "Test" --type html --dry-run
```

## Authentication

```bash
gtm auth login          # Opens browser for Google sign-in
gtm auth status         # Check authentication status
gtm auth logout         # Clear stored credentials
```

## Configuration

Set defaults to avoid repeating flags:

```bash
gtm config setup                          # Interactive setup
gtm config set defaultAccountId 123456    # Set individually
gtm config set defaultContainerId 789
gtm config get                            # Show all settings
```

Environment variables take precedence:

| Variable | Description |
|----------|-------------|
| `GTM_ACCOUNT_ID` | Default account ID |
| `GTM_CONTAINER_ID` | Default container ID |
| `GTM_WORKSPACE_ID` | Default workspace ID |

## Global Flags

| Flag | Description |
|------|-------------|
| `--format json\|table\|compact` | Output format (auto-detects: table for TTY, json for pipes) |
| `--dry-run` | Preview changes without executing |
| `--quiet` | Suppress non-essential output |
| `--no-color` | Disable colored output |

## Resources

| Resource | Commands | Scope |
|----------|----------|-------|
| `accounts` | list, get, update | Account |
| `containers` | list, get, create, update, delete, snippet, lookup, combine, move-tag-id | Account |
| `workspaces` | list, get, create, update, delete, status, sync, create-version, quick-preview, resolve-conflict, export, import | Container |
| `tags` | list, get, create, update, delete, revert | Workspace |
| `triggers` | list, get, create, update, delete, revert | Workspace |
| `variables` | list, get, create, update, delete, revert | Workspace |
| `builtin-variables` | list, create, delete, revert | Workspace |
| `folders` | list, get, create, update, delete, revert, move-entities, entities | Workspace |
| `versions` | list, get, create, update, delete, undelete, publish, set-latest, live | Container |
| `version-headers` | list, latest | Container |
| `environments` | list, get, create, update, delete, reauthorize | Container |
| `destinations` | list, get, link | Container |
| `permissions` | list, get, create, update, delete | Account |
| `clients` | list, get, create, update, delete, revert | Workspace |
| `gtag-configs` | list, get, create, update, delete, revert | Workspace |
| `templates` | list, get, create, update, delete, revert, import | Workspace |
| `transformations` | list, get, create, update, delete, revert | Workspace |
| `zones` | list, get, create, update, delete, revert | Workspace |

### Utility Commands

| Command | Description |
|---------|-------------|
| `setup` | Quick setup workflows (GA4, Facebook Pixel, form tracking) |
| `validate` | Validate workspace resources for common issues (unused triggers, orphan tags, etc.) |
| `changelog` | Compare two container versions and show changes (added/removed/modified) |
| `doctor` | Check environment setup (credentials, auth, config) |
| `config` | Manage default settings |
| `upgrade` | Self-update to latest version |
| `agent guide` | Documentation for AI agents |
| `completions` | Generate shell completions |

## Entity Hierarchy

```
Account
  ├── Container
  │     ├── Workspace
  │     │     ├── Tag
  │     │     ├── Trigger
  │     │     ├── Variable
  │     │     ├── Built-In Variable
  │     │     ├── Folder
  │     │     ├── Client (server-side)
  │     │     ├── Google Tag Config
  │     │     ├── Template
  │     │     ├── Transformation (server-side)
  │     │     └── Zone (server-side)
  │     ├── Version
  │     ├── Version Header
  │     ├── Destination
  │     └── Environment
  └── User Permission
```

## Update Notifications

The CLI automatically checks for new versions once a day (in the background, without blocking). If a newer version is available, a notification is shown:

```
  Update available: v0.1.0 → v0.2.0
  Run `gtm upgrade` to update.
```

Use `--quiet` to suppress update notifications.

## Shell Completions

```bash
gtm completions bash > ~/.local/share/bash-completion/completions/gtm
gtm completions zsh > ~/.zfunc/_gtm
gtm completions fish > ~/.config/fish/completions/gtm.fish
```

## AI Agent Integration

Built for AI agents and LLM tool-use workflows:

- **Structured output** — JSON by default when piped, structured error JSON to stderr
- **Stable exit codes** — `0` success, `1` API error, `2` auth, `3` validation, `4` invalid input
- **Environment diagnostics** — `gtm doctor --format json` for preflight checks
- **Comprehensive guide** — `gtm agent guide` for full documentation
- **Machine-readable spec** — See [`AGENTS.md`](AGENTS.md) for the complete reference

```bash
# Quick setup for agents
gtm doctor --format json          # Verify environment
gtm agent guide                   # Full documentation
gtm tags list --format json       # Structured output
gtm tags create --dry-run ...     # Preview before mutating
```

## Development

```bash
cargo build              # Build
cargo test               # Run tests (63 mock + 11 CLI tests)
cargo clippy             # Lint
cargo fmt                # Format
cargo run -- <command>   # Run in dev mode
```

## License

MIT
