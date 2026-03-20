# GTM CLI

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

## Features

- **Full GTM API v2 coverage** — 20 resource types, 100+ operations
- **Quick setup workflows** — GA4, Facebook Pixel, form tracking in one command
- **Multiple output formats** — Table (terminal), JSON (pipes), compact
- **Auto-detects context** — Table for TTY, JSON when piped
- **Workspace auto-resolution** — No need to specify workspace ID
- **Dry-run mode** — Preview changes before applying
- **Background update checks** — Get notified of new versions
- **Built for AI agents** — Structured JSON output, `gtm agent guide` for docs

## Supported Resources

| Resource | Operations |
|----------|-----------|
| accounts | list, get, update |
| containers | list, get, create, update, delete, snippet, lookup, combine, move-tag-id |
| workspaces | list, get, create, update, delete, status, sync, create-version, quick-preview, resolve-conflict, export, import |
| tags | list, get, create, update, delete, revert |
| triggers | list, get, create, update, delete, revert |
| variables | list, get, create, update, delete, revert |
| builtin-variables | list, create, delete, revert |
| folders | list, get, create, update, delete, revert, move-entities, entities |
| versions | list, get, create, update, delete, undelete, publish, set-latest, live |
| version-headers | list, latest |
| environments | list, get, create, update, delete, reauthorize |
| destinations | list, get, link |
| permissions | list, get, create, update, delete |
| clients | list, get, create, update, delete, revert |
| gtag-configs | list, get, create, update, delete, revert |
| templates | list, get, create, update, delete, revert, import |
| transformations | list, get, create, update, delete, revert |
| zones | list, get, create, update, delete, revert |

## How It Works

This npm package downloads the platform-specific binary from [GitHub Releases](https://github.com/clichedmoog/gtmcli/releases) during `postinstall`. Supported platforms:

- macOS (Apple Silicon, Intel)
- Linux (x86_64, ARM64)
- Windows (x86_64)

## Documentation

Full documentation and source code: [github.com/clichedmoog/gtmcli](https://github.com/clichedmoog/gtmcli)

## License

MIT
