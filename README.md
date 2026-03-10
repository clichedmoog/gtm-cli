# gtm-cli

A command-line interface for the Google Tag Manager API v2 — built for humans and AI agents.

```
gtm <resource> <action> [flags]
```

## Installation

### Quick Install (macOS / Linux)

```bash
curl -fsSL https://github.com/clichedmoog/gtmcli/releases/latest/download/gtm-$(uname -m | sed 's/arm64/aarch64/')-apple-darwin.tar.gz | tar xz -C /usr/local/bin
```

### npm

```bash
npm install -g gtmcli
```

### Homebrew (coming soon)

```bash
brew install clichedmoog/tap/gtm
```

### From source

```bash
git clone https://github.com/clichedmoog/gtmcli.git
cd gtmcli
cargo install --path .
```

## Quick Start

```bash
# Authenticate with Google (opens browser)
gtm auth login

# Set default account/container
gtm config setup

# List tags
gtm tags list

# Create a GA4 tag
gtm setup ga4 --measurement-id G-XXXXXXX
```

## Authentication

### OAuth (default)

```bash
gtm auth login          # Opens browser for Google sign-in
gtm auth status         # Check authentication status
gtm auth logout         # Clear stored credentials
```

Built-in OAuth credentials are included — no setup required.

### Service Account

```bash
# Login with service account key
gtm auth login --service-account /path/to/key.json

# Or via environment variable
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json
gtm accounts list
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
| `containers` | list, get, create, update, delete, snippet, lookup | Account |
| `workspaces` | list, get, create, update, delete, status, sync, create-version, quick-preview, export, import | Container |
| `tags` | list, get, create, update, delete, revert | Workspace |
| `triggers` | list, get, create, update, delete, revert | Workspace |
| `variables` | list, get, create, update, delete, revert | Workspace |
| `builtin-variables` | list, create, delete, revert | Workspace |
| `folders` | list, get, create, update, delete, revert, move-entities, entities | Workspace |
| `versions` | list, get, update, delete, undelete, publish, set-latest, live | Container |
| `version-headers` | list, latest | Container |
| `environments` | list, get, create, update, delete, reauthorize | Container |
| `destinations` | list, get, link | Container |
| `permissions` | list, get, create, update, delete | Account |
| `clients` | list, get, create, update, delete, revert | Workspace |
| `gtag-configs` | list, get, create, update, delete, revert | Workspace |
| `templates` | list, get, create, update, delete, revert, import | Workspace |
| `transformations` | list, get, create, update, delete, revert | Workspace |
| `zones` | list, get | Workspace |

### Utility Commands

| Command | Description |
|---------|-------------|
| `setup` | Quick setup workflows (GA4, Facebook Pixel, form tracking) |
| `config` | Manage default settings |
| `upgrade` | Self-update to latest version |
| `agent guide` | Documentation for AI agents |
| `completions` | Generate shell completions |

## Usage Examples

### Output formats

```bash
# Table (default in terminal)
gtm tags list

# JSON (default when piped)
gtm tags list | jq '.[].name'

# Compact (ID + name only)
gtm tags list --format compact
```

### Creating resources

```bash
# GA4 Event tag
gtm tags create --name "GA4 - Button Click" --type gaawe \
  --firing-trigger-id 2 \
  --params '{"eventName":"button_click","measurementIdOverride":"G-XXXXXXX"}'

# Custom Event trigger
gtm triggers create --name "Button Click" --type customEvent \
  --custom-event-filter "button_click"

# Data Layer variable
gtm variables create --name "User ID" --type v --params '{"name":"userId"}'
```

### Quick setup workflows

```bash
gtm setup ga4 --measurement-id G-XXXXXXX
gtm setup facebook-pixel --pixel-id 1234567890
gtm setup form-tracking --measurement-id G-XXXXXXX
```

### Export & import

```bash
gtm workspaces export -o backup.json
gtm workspaces import -i backup.json
```

### Version management

```bash
gtm workspaces create-version --name "v1.0" --notes "Initial release"
gtm versions publish --version-id 1
gtm versions live
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

### Shell completions

```bash
gtm completions bash > ~/.local/share/bash-completion/completions/gtm
gtm completions zsh > ~/.zfunc/_gtm
gtm completions fish > ~/.config/fish/completions/gtm.fish
```

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

## AI Agent Integration

```bash
gtm agent guide    # Print comprehensive guide for AI agents
```

The CLI outputs structured JSON by default when piped, making it ideal for automation and AI agent workflows.

## Development

```bash
cargo build              # Build
cargo test               # Run tests
cargo clippy             # Lint
cargo fmt                # Format
cargo run -- <command>   # Run in dev mode
```

## License

MIT
