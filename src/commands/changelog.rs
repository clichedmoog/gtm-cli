use clap::{Args, ValueEnum};
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::api::client::GtmApiClient;
use crate::error::Result;
use crate::output::formatter::OutputFormat;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum ChangelogStyle {
    /// Detailed diff table (default)
    Diff,
    /// Deployment note format (title + description)
    Note,
}

#[derive(Args)]
pub struct ChangelogArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    /// Source version ID
    #[arg(long)]
    from: String,
    /// Target version ID (defaults to live/published version)
    #[arg(long)]
    to: Option<String>,
    /// Output style
    #[arg(long, default_value = "diff")]
    style: ChangelogStyle,
}

struct Change {
    change_type: &'static str,
    resource_type: &'static str,
    resource_id: String,
    name: String,
    details: Option<String>,
}

pub async fn handle(
    args: ChangelogArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    let container = format!(
        "accounts/{}/containers/{}",
        args.account_id, args.container_id
    );

    let from_path = format!("{container}/versions/{}", args.from);
    let to_path = match &args.to {
        Some(id) => format!("{container}/versions/{id}"),
        None => format!("{container}/versions:live"),
    };

    let (from_ver, to_ver) = tokio::join!(client.get(&from_path), client.get(&to_path));
    let from_ver = from_ver?;
    let to_ver = to_ver?;

    let from_id = &args.from;
    let to_id = args.to.as_deref().unwrap_or_else(|| {
        to_ver
            .get("containerVersionId")
            .and_then(|v| v.as_str())
            .unwrap_or("live")
    });

    let changes = collect_changes(&from_ver, &to_ver);

    let added = changes.iter().filter(|c| c.change_type == "added").count();
    let removed = changes
        .iter()
        .filter(|c| c.change_type == "removed")
        .count();
    let modified = changes
        .iter()
        .filter(|c| c.change_type == "modified")
        .count();

    match args.style {
        ChangelogStyle::Diff => {
            render_diff(&changes, from_id, to_id, added, removed, modified, format)
        }
        ChangelogStyle::Note => {
            render_note(&changes, to_id, added, removed, modified, format)
        }
    }

    Ok(())
}

fn collect_changes(from_ver: &Value, to_ver: &Value) -> Vec<Change> {
    let mut changes = Vec::new();

    for (resource_type, key, id_field) in &[
        ("tag", "tag", "tagId"),
        ("trigger", "trigger", "triggerId"),
        ("variable", "variable", "variableId"),
    ] {
        let from_items = extract_map(from_ver, key, id_field);
        let to_items = extract_map(to_ver, key, id_field);

        for (id, item) in &to_items {
            if !from_items.contains_key(id) {
                changes.push(Change {
                    change_type: "added",
                    resource_type,
                    resource_id: id.clone(),
                    name: get_str(item, "name"),
                    details: None,
                });
            }
        }

        for (id, item) in &from_items {
            if !to_items.contains_key(id) {
                changes.push(Change {
                    change_type: "removed",
                    resource_type,
                    resource_id: id.clone(),
                    name: get_str(item, "name"),
                    details: None,
                });
            }
        }

        for (id, from_item) in &from_items {
            if let Some(to_item) = to_items.get(id) {
                let from_fp = from_item.get("fingerprint").and_then(|v| v.as_str());
                let to_fp = to_item.get("fingerprint").and_then(|v| v.as_str());
                if from_fp != to_fp {
                    let details = build_diff_details(from_item, to_item);
                    changes.push(Change {
                        change_type: "modified",
                        resource_type,
                        resource_id: id.clone(),
                        name: get_str(to_item, "name"),
                        details,
                    });
                }
            }
        }
    }

    changes
}

fn render_diff(
    changes: &[Change],
    from_id: &str,
    to_id: &str,
    added: usize,
    removed: usize,
    modified: usize,
    format: &OutputFormat,
) {
    match format {
        OutputFormat::Json => {
            let json_changes: Vec<Value> = changes
                .iter()
                .map(|c| {
                    json!({
                        "changeType": c.change_type,
                        "resourceType": c.resource_type,
                        "resourceId": c.resource_id,
                        "name": c.name,
                        "details": c.details,
                    })
                })
                .collect();
            let output = json!({
                "fromVersion": from_id,
                "toVersion": to_id,
                "changes": json_changes,
                "summary": { "added": added, "removed": removed, "modified": modified },
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Table | OutputFormat::Compact => {
            if changes.is_empty() {
                println!("No changes between version {from_id} and {to_id}.");
            } else {
                use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_content_arrangement(ContentArrangement::Dynamic);
                table.set_header(["Change", "Type", "ID", "Name", "Details"]);
                for c in changes {
                    table.add_row([
                        c.change_type,
                        c.resource_type,
                        &c.resource_id,
                        &c.name,
                        c.details.as_deref().unwrap_or(""),
                    ]);
                }
                println!("{table}");
                println!(
                    "\nSummary: {} added, {} removed, {} modified",
                    added, removed, modified
                );
            }
        }
    }
}

fn render_note(
    changes: &[Change],
    to_id: &str,
    added: usize,
    removed: usize,
    modified: usize,
    format: &OutputFormat,
) {
    let title = build_title(to_id, changes);
    let description = build_description(changes);

    match format {
        OutputFormat::Json => {
            let output = json!({
                "title": title,
                "description": description,
                "summary": { "added": added, "removed": removed, "modified": modified },
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Table | OutputFormat::Compact => {
            println!("{title}");
            if !description.is_empty() {
                println!();
                println!("{description}");
            }
        }
    }
}

fn build_title(to_id: &str, changes: &[Change]) -> String {
    let added = changes.iter().filter(|c| c.change_type == "added").count();
    let removed = changes.iter().filter(|c| c.change_type == "removed").count();
    let modified = changes.iter().filter(|c| c.change_type == "modified").count();

    if added == 0 && removed == 0 && modified == 0 {
        return format!("v{to_id}: no changes");
    }

    // Build human-readable parts like "3 tags added", "1 trigger modified"
    let mut parts = Vec::new();
    for (change_type, verb) in &[("added", "added"), ("modified", "modified"), ("removed", "removed")] {
        for rtype in &["tag", "trigger", "variable"] {
            let n = changes
                .iter()
                .filter(|c| c.change_type == *change_type && c.resource_type == *rtype)
                .count();
            if n > 0 {
                let label = if n == 1 { rtype.to_string() } else { format!("{rtype}s") };
                parts.push(format!("{n} {label} {verb}"));
            }
        }
    }

    // If too many parts, summarize with "and more"
    let title_body = if parts.len() <= 3 {
        parts.join(", ")
    } else {
        let shown: Vec<_> = parts.iter().take(2).cloned().collect();
        format!("{} and more", shown.join(", "))
    };

    format!("v{to_id}: {title_body}")
}

fn build_description(changes: &[Change]) -> String {
    let mut sections = Vec::new();

    let added: Vec<_> = changes.iter().filter(|c| c.change_type == "added").collect();
    let modified: Vec<_> = changes
        .iter()
        .filter(|c| c.change_type == "modified")
        .collect();
    let removed: Vec<_> = changes
        .iter()
        .filter(|c| c.change_type == "removed")
        .collect();

    if !added.is_empty() {
        let mut lines = vec!["[Added]".to_string()];
        for c in &added {
            lines.push(format!("  {} ({})", c.name, c.resource_type));
        }
        sections.push(lines.join("\n"));
    }

    if !modified.is_empty() {
        let mut lines = vec!["[Modified]".to_string()];
        for c in &modified {
            if let Some(details) = &c.details {
                lines.push(format!("  {} ({}) - {}", c.name, c.resource_type, details));
            } else {
                lines.push(format!("  {} ({})", c.name, c.resource_type));
            }
        }
        sections.push(lines.join("\n"));
    }

    if !removed.is_empty() {
        let mut lines = vec!["[Removed]".to_string()];
        for c in &removed {
            lines.push(format!("  {} ({})", c.name, c.resource_type));
        }
        sections.push(lines.join("\n"));
    }

    sections.join("\n\n")
}

fn extract_map(version: &Value, key: &str, id_field: &str) -> HashMap<String, Value> {
    version
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|item| {
                    item.get(id_field)
                        .and_then(|v| v.as_str())
                        .map(|id| (id.to_string(), item.clone()))
                })
                .collect()
        })
        .unwrap_or_default()
}

fn get_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string()
}

fn build_diff_details(from: &Value, to: &Value) -> Option<String> {
    let mut diffs = Vec::new();

    let from_name = from.get("name").and_then(|v| v.as_str()).unwrap_or("");
    let to_name = to.get("name").and_then(|v| v.as_str()).unwrap_or("");
    if from_name != to_name {
        diffs.push(format!("name: '{}' -> '{}'", from_name, to_name));
    }

    let from_type = from.get("type").and_then(|v| v.as_str()).unwrap_or("");
    let to_type = to.get("type").and_then(|v| v.as_str()).unwrap_or("");
    if from_type != to_type {
        diffs.push(format!("type: '{}' -> '{}'", from_type, to_type));
    }

    if diffs.is_empty() {
        Some("configuration changed".into())
    } else {
        Some(diffs.join(", "))
    }
}
