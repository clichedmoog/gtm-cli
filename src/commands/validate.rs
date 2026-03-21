use clap::Args;
use serde_json::{json, Value};
use std::collections::{HashMap, HashSet};

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::OutputFormat;

#[derive(Args)]
pub struct ValidateArgs {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
}

struct Issue {
    severity: &'static str,
    rule: &'static str,
    resource_type: &'static str,
    resource_id: String,
    resource_name: String,
    message: String,
}

pub async fn handle(
    args: ValidateArgs,
    client: &GtmApiClient,
    format: &OutputFormat,
) -> Result<()> {
    let ws_id = resolve_workspace(
        client,
        &args.account_id,
        &args.container_id,
        args.workspace_id.as_deref(),
    )
    .await?;
    let base = format!(
        "accounts/{}/containers/{}/workspaces/{}",
        args.account_id, args.container_id, ws_id
    );

    // Fetch all resources concurrently
    let tags_path = format!("{base}/tags");
    let triggers_path = format!("{base}/triggers");
    let variables_path = format!("{base}/variables");
    let folders_path = format!("{base}/folders");
    let (tags_res, triggers_res, variables_res, folders_res) = tokio::join!(
        client.get_all(&tags_path),
        client.get_all(&triggers_path),
        client.get_all(&variables_path),
        client.get_all(&folders_path),
    );

    let tags = extract_array(&tags_res?, "tag");
    let triggers = extract_array(&triggers_res?, "trigger");
    let variables = extract_array(&variables_res?, "variable");
    let folders = extract_array(&folders_res?, "folder");

    let mut issues = Vec::new();

    // Rule 1: no-firing-trigger — tags without firing triggers
    for tag in &tags {
        let has_firing = tag
            .get("firingTriggerId")
            .and_then(|v| v.as_array())
            .is_some_and(|a| !a.is_empty());
        if !has_firing {
            issues.push(Issue {
                severity: "error",
                rule: "no-firing-trigger",
                resource_type: "tag",
                resource_id: get_str(tag, "tagId"),
                resource_name: get_str(tag, "name"),
                message: "Tag has no firing triggers".into(),
            });
        }
    }

    // Rule 2: unused-trigger — triggers not referenced by any tag
    let mut referenced_triggers: HashSet<String> = HashSet::new();
    for tag in &tags {
        for key in &["firingTriggerId", "blockingTriggerId"] {
            if let Some(arr) = tag.get(*key).and_then(|v| v.as_array()) {
                for id in arr {
                    if let Some(s) = id.as_str() {
                        referenced_triggers.insert(s.to_string());
                    }
                }
            }
        }
    }
    for trigger in &triggers {
        let tid = get_str(trigger, "triggerId");
        if !referenced_triggers.contains(&tid) {
            issues.push(Issue {
                severity: "warning",
                rule: "unused-trigger",
                resource_type: "trigger",
                resource_id: tid,
                resource_name: get_str(trigger, "name"),
                message: "Trigger not referenced by any tag".into(),
            });
        }
    }

    // Rule 3: unused-variable — variables not referenced as {{name}} anywhere
    let all_text = build_searchable_text(&tags, &triggers);
    for var in &variables {
        let name = get_str(var, "name");
        let pattern = format!("{{{{{name}}}}}");
        if !all_text.contains(&pattern) {
            issues.push(Issue {
                severity: "warning",
                rule: "unused-variable",
                resource_type: "variable",
                resource_id: get_str(var, "variableId"),
                resource_name: name,
                message: "Variable not referenced by any tag or trigger".into(),
            });
        }
    }

    // Rule 4: empty-folder — folders with no children
    let used_folders: HashSet<String> = tags
        .iter()
        .chain(triggers.iter())
        .chain(variables.iter())
        .filter_map(|r| r.get("parentFolderId").and_then(|v| v.as_str()))
        .map(String::from)
        .collect();
    for folder in &folders {
        let fid = get_str(folder, "folderId");
        if !used_folders.contains(&fid) {
            issues.push(Issue {
                severity: "warning",
                rule: "empty-folder",
                resource_type: "folder",
                resource_id: fid,
                resource_name: get_str(folder, "name"),
                message: "Folder has no items".into(),
            });
        }
    }

    // Rule 5: duplicate-tag-name — tags sharing the same name
    let mut name_counts: HashMap<String, Vec<String>> = HashMap::new();
    for tag in &tags {
        let name = get_str(tag, "name");
        let id = get_str(tag, "tagId");
        name_counts.entry(name).or_default().push(id);
    }
    for (name, ids) in &name_counts {
        if ids.len() > 1 {
            for id in ids {
                issues.push(Issue {
                    severity: "warning",
                    rule: "duplicate-tag-name",
                    resource_type: "tag",
                    resource_id: id.clone(),
                    resource_name: name.clone(),
                    message: format!("Duplicate tag name ({}x)", ids.len()),
                });
            }
        }
    }

    // Output
    let errors = issues.iter().filter(|i| i.severity == "error").count();
    let warnings = issues.iter().filter(|i| i.severity == "warning").count();
    let total = issues.len();

    match format {
        OutputFormat::Json => {
            let json_issues: Vec<Value> = issues
                .iter()
                .map(|i| {
                    json!({
                        "severity": i.severity,
                        "rule": i.rule,
                        "resourceType": i.resource_type,
                        "resourceId": i.resource_id,
                        "resourceName": i.resource_name,
                        "message": i.message,
                    })
                })
                .collect();
            let output = json!({
                "issues": json_issues,
                "summary": { "total": total, "errors": errors, "warnings": warnings },
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Table | OutputFormat::Compact => {
            if issues.is_empty() {
                println!("No issues found.");
            } else {
                use comfy_table::{presets::UTF8_FULL_CONDENSED, ContentArrangement, Table};
                let mut table = Table::new();
                table
                    .load_preset(UTF8_FULL_CONDENSED)
                    .set_content_arrangement(ContentArrangement::Dynamic);
                table.set_header(["Severity", "Rule", "Type", "ID", "Name", "Message"]);
                for i in &issues {
                    table.add_row([
                        i.severity,
                        i.rule,
                        i.resource_type,
                        &i.resource_id,
                        &i.resource_name,
                        &i.message,
                    ]);
                }
                println!("{table}");
                println!(
                    "\nSummary: {} issue(s) — {} error(s), {} warning(s)",
                    total, errors, warnings
                );
            }
        }
    }

    if errors > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn extract_array(value: &Value, key: &str) -> Vec<Value> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default()
}

fn get_str(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string()
}

/// Serialize all tags and triggers into a single string for variable reference search.
fn build_searchable_text(tags: &[Value], triggers: &[Value]) -> String {
    let mut text = String::new();
    for item in tags.iter().chain(triggers.iter()) {
        text.push_str(&item.to_string());
        text.push('\n');
    }
    text
}
