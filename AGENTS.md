# AGENTS.md — GTM CLI for AI Agents

> Machine-readable reference for AI agents and LLM tool-use integrations.
> For interactive guide, run: `gtm agent guide`

## Quick Start

```bash
# 1. Verify environment
gtm doctor --format json

# 2. Authenticate (service account for non-interactive use)
gtm auth login --service-account /path/to/key.json
# Or: export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json

# 3. Set defaults
export GTM_ACCOUNT_ID=123456
export GTM_CONTAINER_ID=789

# 4. Start working
gtm tags list
```

## Exit Codes

| Code | Meaning | Action |
|------|---------|--------|
| 0 | Success | Proceed |
| 1 | API / general error | Check error message, retry if transient |
| 2 | Authentication error | Run `gtm auth login` |
| 3 | Validation error | Review `gtm validate` output |
| 4 | Invalid input | Fix parameters / JSON |

## Error Format

When stderr is piped (non-TTY), errors are structured JSON:

```json
{
  "error": {
    "code": 2,
    "type": "auth_required",
    "message": "Authentication required. Run `gtm auth login` to authenticate."
  }
}
```

Error types: `auth_required`, `credentials_not_found`, `token_refresh_failed`, `api_error`, `invalid_params`, `validation_failed`, `http_error`, `io_error`, `json_error`

## Output Format

- **Piped stdout** → JSON (default)
- **TTY stdout** → Table (human-readable)
- Force JSON: `--format json`
- Compact (ID + name): `--format compact`

List endpoints return unwrapped arrays: `[{...}, {...}]`
Single resource endpoints return objects: `{...}`

## Global Flags

| Flag | Description |
|------|-------------|
| `--format json\|table\|compact` | Output format |
| `--dry-run` | Preview mutations (returns what would be sent) |
| `--quiet` | Suppress non-essential stderr output |
| `--no-color` | Disable ANSI colors |

## Environment Variables

| Variable | Description |
|----------|-------------|
| `GTM_ACCOUNT_ID` | Default account ID (overrides config) |
| `GTM_CONTAINER_ID` | Default container ID |
| `GTM_WORKSPACE_ID` | Default workspace ID |
| `GOOGLE_APPLICATION_CREDENTIALS` | Path to service account key |

## CRUD Pattern

All workspace-scoped resources follow the same pattern:

```
gtm <resource> list   [--name <FILTER>] [--type <TYPE>]
gtm <resource> get    --<resource>-id <ID>
gtm <resource> create --name <NAME> [--type <TYPE>] [--params '<JSON>' | --params-file <FILE>]
gtm <resource> update --<resource>-id <ID> [--name <NAME>] [--params '<JSON>' | --params-file <FILE>]
gtm <resource> delete --<resource>-id <ID> --force
gtm <resource> revert --<resource>-id <ID>
```

### List Filters

`--name` does case-insensitive substring matching. `--type` is exact match.

```bash
gtm tags list --name "GA4"                  # Tags containing "GA4"
gtm tags list --type gaawe                  # GA4 event tags only
gtm variables list --type v                 # Data layer variables
gtm triggers list --name "click"            # Triggers with "click" in name
```

### File-based Parameters

Use `--params-file` (or `--filter-file` for triggers) to avoid shell escaping issues, especially for Custom HTML tags:

```bash
gtm tags create --name "Tracking Script" --type html --params-file tag.json
gtm tags update --tag-id 123 --params-file updated-tag.json
```

Resources: `tags`, `triggers`, `variables`, `folders`, `templates`, `clients`, `gtag-configs`, `transformations`, `zones`, `builtin-variables`

## Parameters

The `--params` flag accepts a JSON string. It is automatically converted to GTM's nested parameter format.

```bash
# Simple key-value
gtm tags create --name "GA4 Config" --type gaawc \
  --params '{"measurementId": "G-XXXXXXX"}'

# GA4 event tag with event parameters
gtm tags create --name "GA4 Event" --type gaawe \
  --params '{"measurementId": "G-XXXXXXX", "eventName": "purchase", "eventParameters": [{"name": "value", "value": "{{dlv - value}}"}]}'
```

For `gaawe` (GA4 event) tags, `eventParameters` is automatically converted to GTM's `eventSettingsTable` format.

## Resource Hierarchy

```
Account (--account-id)
  └── Container (--container-id)
        ├── Workspace (--workspace-id, auto-resolved if omitted)
        │     ├── Tag (--tag-id)
        │     ├── Trigger (--trigger-id)
        │     ├── Variable (--variable-id)
        │     ├── Folder (--folder-id)
        │     ├── Template (--template-id)
        │     ├── Client (--client-id)
        │     ├── Google Tag Config (--gtag-config-id)
        │     ├── Transformation (--transformation-id)
        │     └── Zone (--zone-id)
        ├── Version (--version-id)
        ├── Environment (--environment-id)
        └── Destination (--destination-id)
```

## Common Workflows

### Create and publish a GA4 setup
```bash
gtm setup ga4 --measurement-id G-XXXXXXX
gtm workspaces create-version --name "Add GA4"
gtm versions publish --version-id <ID>
```

### Validate before publishing
```bash
gtm validate --format json
# Exit code 3 if errors found — do not publish
```

### Compare versions before/after deploy
```bash
gtm changelog --from <OLD_VERSION> --to <NEW_VERSION> --format json
```

### Export workspace as backup
```bash
gtm workspaces export -o backup.json
```

## Safety

- All `delete` commands require `--force` flag
- Use `--dry-run` to preview any mutation
- Rate limiting (HTTP 429) is automatically retried with exponential backoff
- Workspace ID is auto-resolved if omitted (uses first available workspace)

## Diagnostics

```bash
gtm doctor              # Check credentials, auth, config
gtm doctor --format json  # Machine-readable diagnostics
```
