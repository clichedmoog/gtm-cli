use clap::{Args, Subcommand};

use crate::error::Result;

#[derive(Args)]
pub struct AgentArgs {
    #[command(subcommand)]
    pub action: AgentAction,
}

#[derive(Subcommand)]
pub enum AgentAction {
    /// Print comprehensive guide for AI agents
    Guide,
}

pub fn handle(args: AgentArgs) -> Result<()> {
    match args.action {
        AgentAction::Guide => {
            print!("{}", AGENT_GUIDE);
        }
    }
    Ok(())
}

const AGENT_GUIDE: &str = r#"# GTM CLI — AI Agent Guide

## Overview

`gtm` is a CLI for managing Google Tag Manager (GTM) resources via the GTM API v2.
It supports all GTM entity types and provides structured JSON output ideal for programmatic use.

## Authentication

```bash
# OAuth (interactive — opens browser)
gtm auth login

# Service account (non-interactive — ideal for CI/CD and agents)
gtm auth login --service-account /path/to/key.json

# Or via environment variable
export GOOGLE_APPLICATION_CREDENTIALS=/path/to/key.json

# Check status
gtm auth status
```

## Global Flags

- `--format json|table|compact` — Output format (auto-detects: json when piped, table for TTY). Use `json` for programmatic access.
- `--dry-run` — Preview mutating operations without executing them.
- `--quiet` — Suppress non-essential output (update checks, warnings).
- `--no-color` — Disable colored output.

## Configuration Defaults

Set defaults to avoid repeating `--account-id`, `--container-id`, `--workspace-id`:

```bash
gtm config setup                          # Interactive setup
gtm config set defaultAccountId 123456    # Set individually
gtm config set defaultContainerId 789
gtm config set defaultWorkspaceId 1
gtm config get                             # Show all
```

Environment variables take precedence: `GTM_ACCOUNT_ID`, `GTM_CONTAINER_ID`, `GTM_WORKSPACE_ID`.

## Common Workflows

### List all tags in a workspace
```bash
gtm tags list --account-id 123 --container-id 456
```

### Create a GA4 tag with quick setup
```bash
gtm setup ga4 --measurement-id G-XXXXXXXX --account-id 123 --container-id 456
```

### Export and import workspace
```bash
gtm workspaces export --account-id 123 --container-id 456 -o backup.json
gtm workspaces import --account-id 123 --container-id 456 -i backup.json
```

### Create a version and publish
```bash
gtm workspaces create-version --name "v1.0" --notes "Initial release" \
  --account-id 123 --container-id 456 --workspace-id 1
gtm versions publish --version-id 42 --account-id 123 --container-id 456
```

### Check workspace changes
```bash
gtm workspaces status --account-id 123 --container-id 456 --workspace-id 1
```

## Resource Types

| Command              | Resource                  | Scope       |
|----------------------|---------------------------|-------------|
| `accounts`           | GTM Accounts              | Account     |
| `containers`         | Containers                | Account     |
| `workspaces`         | Workspaces                | Container   |
| `tags`               | Tags                      | Workspace   |
| `triggers`           | Triggers                  | Workspace   |
| `variables`          | Variables                 | Workspace   |
| `folders`            | Folders                   | Workspace   |
| `templates`          | Custom Templates          | Workspace   |
| `versions`           | Container Versions        | Container   |
| `version-headers`    | Version Headers (summary) | Container   |
| `environments`       | Environments              | Container   |
| `permissions`        | User Permissions          | Account     |
| `clients`            | Clients (sGTM)            | Workspace   |
| `transformations`    | Transformations (sGTM)    | Workspace   |
| `zones`              | Zones (sGTM)              | Workspace   |
| `destinations`       | Destinations              | Container   |
| `gtag-configs`       | Google Tag Configs        | Workspace   |
| `builtin-variables`  | Built-in Variables        | Workspace   |

## CRUD Pattern

All workspace-scoped resources follow the same pattern:

```bash
gtm <resource> list   [--account-id --container-id]
gtm <resource> get    [--account-id --container-id --<resource>-id]
gtm <resource> create [--account-id --container-id --name ... --params '{}']
gtm <resource> update [--account-id --container-id --<resource>-id --name ...]
gtm <resource> delete [--account-id --container-id --<resource>-id --force]
gtm <resource> revert [--account-id --container-id --<resource>-id]
```

## Parameters Format

Tags, triggers, and variables accept `--params` as a JSON string:

```bash
gtm tags create --name "Event Tag" --type gaawe \
  --params '{"measurementId": "G-XXXXX", "eventName": "purchase"}'
```

The JSON is automatically converted to GTM's nested parameter format.

## Safety

- All `delete` commands require `--force` flag.
- Use `--dry-run` to preview changes before applying.

## Output

Default output is JSON when piped (table in terminal). List responses are unwrapped arrays:

```bash
gtm tags list | jq '.[].name'
gtm containers list | jq '.[] | select(.publicId == "GTM-XXXXX")'
gtm tags list --format compact   # ID + name only
```

## Diagnostics

```bash
gtm doctor              # Check credentials, auth, config
gtm doctor --format json  # Machine-readable diagnostics
```

## Error Handling

Exit codes:
- 0: Success
- 1: API / general error
- 2: Authentication error → run `gtm auth login`
- 3: Validation error → review `gtm validate` output
- 4: Invalid input → fix parameters / JSON

When stderr is piped (non-TTY), errors are structured JSON:
```json
{"error": {"code": 2, "type": "auth_required", "message": "..."}}
```

Rate limiting (429) is automatically retried with exponential backoff.
"#;
