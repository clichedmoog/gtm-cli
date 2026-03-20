# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

`gtm` is a Rust CLI tool for managing Google Tag Manager (GTM) resources via the GTM API v2. It provides CRUD operations for all GTM entity types (accounts, containers, workspaces, tags, triggers, variables, etc.) plus quick-setup workflows for common configurations like GA4 and Facebook Pixel.

## Build & Test Commands

```bash
cargo build              # Build
cargo run -- <args>      # Run (e.g., cargo run -- tags list --account-id 123 --container-id 456)
cargo test               # Run all tests
cargo test <test_name>   # Run a single test
cargo clippy             # Lint
cargo fmt                # Format
```

The binary is named `gtm` (configured in Cargo.toml `[[bin]]`).

## Architecture

### Module Structure

- **`src/main.rs`** — CLI entry point using `clap` derive. Defines `Commands` enum mapping subcommands to handlers. All commands except `auth` and `completions` require an authenticated `GtmApiClient`.
- **`src/api/`** — API layer
  - `client.rs` — `GtmApiClient` wraps `reqwest::Client` with auth headers, retry logic (exponential backoff on 429), and `--dry-run` support for mutating operations.
  - `params.rs` — Converts JSON values to GTM's wire format (`GtmParameter` enum with Template/List/Map variants). Mirrors the TypeScript `convertParameterValue` pattern.
  - `workspace.rs` — `resolve_workspace()` auto-resolves workspace ID: uses provided ID, falls back to first existing workspace, or creates a "Default Workspace".
- **`src/auth/`** — OAuth2 flow
  - `oauth.rs` — Full OAuth2 login (local HTTP server for redirect), token refresh, and `ensure_valid_token()` for auto-refresh.
  - `token_store.rs` — Reads/writes credentials and tokens from `~/.config/gtm/`.
- **`src/config.rs`** — Config loading. Credentials/token paths from env vars (`GTM_CREDENTIALS_FILE`, `GTM_TOKEN_FILE`) or defaults under `~/.config/gtm/`.
- **`src/update_check.rs`** — Background update checker. Runs via `tokio::spawn` on startup, checks GitHub releases once per day, caches result in `~/.config/gtm/update-check.json`. Uses `semver` crate for version comparison.
- **`src/error.rs`** — `GtmError` enum using `thiserror`. `exit_with_message()` prints user-friendly errors.
- **`src/output/`** — Output formatting with `--format json|table` (global flag, default json).
  - `formatter.rs` — Dispatches to JSON pretty-print or table rendering.
  - `table.rs` — Table rendering with `comfy-table`.
- **`src/commands/`** — One file per GTM resource type. Each follows the same pattern:
  - `*Args` struct with `#[command(subcommand)]` for CRUD actions (list/get/create/update/delete/revert)
  - `WorkspaceFlags` with `--account-id`, `--container-id`, `--workspace-id` (all support env vars `GTM_ACCOUNT_ID`, `GTM_CONTAINER_ID`, `GTM_WORKSPACE_ID`)
  - `handle()` async function dispatching to the appropriate API call

### Command Pattern

Every resource command follows the same structure. When adding a new resource:
1. Create `src/commands/<resource>.rs` with Args/Action enums and `handle()` function
2. Add the module to `src/commands/mod.rs`
3. Add the variant to the `Commands` enum in `main.rs`

### Key Design Decisions

- **All API calls go through `GtmApiClient`** which handles auth, retries, and dry-run uniformly.
- **Workspace ID is optional** — if omitted, `resolve_workspace()` finds or creates one automatically.
- **Parameters use `--params` as a JSON string** parsed into GTM's nested parameter format via `params_from_json()`.
- **Global flags** (`--format`, `--dry-run`) are defined on the root `Cli` struct and threaded through to handlers.
- **Testability** — `GtmApiClient` supports `GTM_API_BASE` env var to override the API base URL and bypasses real auth when set, enabling mock server testing with `wiremock`.

### Test Structure

- **`tests/mock_api.rs`** — 63 integration tests using `wiremock` to simulate GTM API responses. Covers all resource types, CRUD operations, pagination, dry-run, error handling, and output formats.
- **`tests/cli_basic.rs`** — Basic CLI tests (help, version, completions, flag validation).
- **`tests/integration.rs`** — Integration tests against real GTM API (ignored by default, requires credentials).
