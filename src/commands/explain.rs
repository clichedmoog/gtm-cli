use clap::Args;
use serde_json::{json, Value};
use std::collections::HashMap;

use crate::api::client::GtmApiClient;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::OutputFormat;

#[derive(Args)]
pub struct ExplainArgs {
    /// Tag ID to explain
    #[arg(long)]
    pub tag_id: String,
    /// Show other tags sharing the same triggers
    #[arg(long)]
    pub reverse: bool,
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
}

struct TriggerInfo {
    id: String,
    name: String,
    trigger_type: String,
    conditions: Vec<Condition>,
    shared_by: Vec<SharedTag>,
}

struct Condition {
    variable: String,
    operator: String,
    value: String,
}

#[derive(Clone)]
struct SharedTag {
    id: String,
    name: String,
    tag_type: String,
}

struct VarInfo {
    name: String,
    var_type: String,
    id: String,
}

pub async fn handle(args: ExplainArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
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

    // Fetch tag + all triggers + all variables concurrently
    // Also fetch all tags if --reverse is requested
    let tag_path = format!("{base}/tags/{}", args.tag_id);
    let triggers_path = format!("{base}/triggers");
    let variables_path = format!("{base}/variables");
    let tags_path = format!("{base}/tags");

    let (tag_res, triggers_res, variables_res, tags_res) = tokio::join!(
        client.get(&tag_path),
        client.get_all(&triggers_path),
        client.get_all(&variables_path),
        async {
            if args.reverse {
                client.get_all(&tags_path).await
            } else {
                Ok(json!({}))
            }
        },
    );

    let tag = tag_res?;
    let triggers_data = triggers_res?;
    let variables_data = variables_res?;
    let tags_data = tags_res?;

    let tag_name = str_field(&tag, "name");
    let tag_type = str_field(&tag, "type");
    let tag_id = str_field(&tag, "tagId");

    // Build trigger index
    let all_triggers: Vec<&Value> = triggers_data
        .get("trigger")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let trigger_map: HashMap<&str, &Value> = all_triggers
        .iter()
        .filter_map(|t| {
            t.get("triggerId")
                .and_then(|v| v.as_str())
                .map(|id| (id, *t))
        })
        .collect();

    // Build reverse index: trigger_id → tags that use it (excluding current tag)
    let reverse_map = if args.reverse {
        build_reverse_map(&tags_data, &tag_id)
    } else {
        HashMap::new()
    };

    // Resolve firing triggers
    let firing_ids = id_array(&tag, "firingTriggerId");
    let firing: Vec<TriggerInfo> = firing_ids
        .iter()
        .map(|id| resolve_trigger(id, &trigger_map, &reverse_map))
        .collect();

    // Resolve blocking triggers
    let blocking_ids = id_array(&tag, "blockingTriggerId");
    let blocking: Vec<TriggerInfo> = blocking_ids
        .iter()
        .map(|id| resolve_trigger(id, &trigger_map, &reverse_map))
        .collect();

    // Extract referenced variables
    let tag_json = tag.to_string();
    let referenced_vars = extract_variable_refs(&tag_json);

    let all_variables: Vec<&Value> = variables_data
        .get("variable")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect())
        .unwrap_or_default();
    let var_map: HashMap<&str, &Value> = all_variables
        .iter()
        .filter_map(|v| {
            v.get("name")
                .and_then(|n| n.as_str())
                .map(|name| (name, *v))
        })
        .collect();

    let variables: Vec<VarInfo> = referenced_vars
        .iter()
        .map(|name| {
            if let Some(v) = var_map.get(name.as_str()) {
                VarInfo {
                    name: name.clone(),
                    var_type: str_field(v, "type"),
                    id: str_field(v, "variableId"),
                }
            } else {
                VarInfo {
                    name: name.clone(),
                    var_type: "built-in".into(),
                    id: "-".into(),
                }
            }
        })
        .collect();

    let params = extract_key_params(&tag);

    match format {
        OutputFormat::Json => {
            let trigger_json = |t: &TriggerInfo| {
                let mut obj = json!({
                    "id": t.id,
                    "name": t.name,
                    "type": t.trigger_type,
                    "conditions": t.conditions.iter().map(|c| json!({
                        "variable": c.variable,
                        "operator": c.operator,
                        "value": c.value,
                    })).collect::<Vec<_>>(),
                });
                if !t.shared_by.is_empty() {
                    obj["sharedBy"] = json!(t
                        .shared_by
                        .iter()
                        .map(|s| json!({
                            "id": s.id, "name": s.name, "type": s.tag_type,
                        }))
                        .collect::<Vec<_>>());
                }
                obj
            };
            let output = json!({
                "tag": { "id": tag_id, "name": tag_name, "type": tag_type },
                "parameters": params,
                "firingTriggers": firing.iter().map(trigger_json).collect::<Vec<_>>(),
                "blockingTriggers": blocking.iter().map(trigger_json).collect::<Vec<_>>(),
                "referencedVariables": variables.iter().map(|v| json!({
                    "name": v.name, "type": v.var_type, "id": v.id,
                })).collect::<Vec<_>>(),
            });
            println!("{}", serde_json::to_string_pretty(&output).unwrap());
        }
        OutputFormat::Table | OutputFormat::Compact => {
            println!("Tag: {} (id: {}, type: {})", tag_name, tag_id, tag_type);

            if !params.is_empty() {
                println!("\nParameters:");
                for (k, v) in &params {
                    println!("  {k}: {v}");
                }
            }

            print_triggers_section("Firing Triggers", &firing);

            if !blocking.is_empty() {
                print_triggers_section("Blocking Triggers", &blocking);
            }

            println!("\nReferenced Variables:");
            if variables.is_empty() {
                println!("  (none)");
            } else {
                for v in &variables {
                    if v.id == "-" {
                        println!("  {{{{{}}}}} ({})", v.name, v.var_type);
                    } else {
                        println!("  {{{{{}}}}} (id: {}, type: {})", v.name, v.id, v.var_type);
                    }
                }
            }
        }
    }

    Ok(())
}

fn print_triggers_section(title: &str, triggers: &[TriggerInfo]) {
    println!("\n{title}:");
    if triggers.is_empty() {
        println!("  (none)");
        return;
    }
    for t in triggers {
        println!("  [{}] {} ({})", t.id, t.name, t.trigger_type);
        for c in &t.conditions {
            println!(
                "        Condition: {} {} {}",
                c.variable, c.operator, c.value
            );
        }
        if !t.shared_by.is_empty() {
            println!("        Also used by:");
            for s in &t.shared_by {
                println!("          [{}] {} ({})", s.id, s.name, s.tag_type);
            }
        }
    }
}

fn resolve_trigger(
    id: &str,
    map: &HashMap<&str, &Value>,
    reverse_map: &HashMap<String, Vec<SharedTag>>,
) -> TriggerInfo {
    let shared_by = reverse_map.get(id).cloned().unwrap_or_default();

    if let Some(t) = map.get(id) {
        TriggerInfo {
            id: id.into(),
            name: str_field(t, "name"),
            trigger_type: str_field(t, "type"),
            conditions: extract_conditions(t),
            shared_by,
        }
    } else {
        TriggerInfo {
            id: id.into(),
            name: "(unknown)".into(),
            trigger_type: "-".into(),
            conditions: vec![],
            shared_by,
        }
    }
}

fn extract_conditions(trigger: &Value) -> Vec<Condition> {
    let mut conditions = Vec::new();
    // GTM triggers store conditions in these fields
    for field in ["customEventFilter", "filter", "autoEventFilter"] {
        if let Some(arr) = trigger.get(field).and_then(|v| v.as_array()) {
            for filter in arr {
                if let Some(params) = filter.get("parameter").and_then(|v| v.as_array()) {
                    let mut variable = String::new();
                    let mut value = String::new();
                    for param in params {
                        let key = param.get("key").and_then(|v| v.as_str()).unwrap_or("");
                        let val = param.get("value").and_then(|v| v.as_str()).unwrap_or("");
                        match key {
                            "arg0" => variable = val.to_string(),
                            "arg1" => value = val.to_string(),
                            _ => {}
                        }
                    }
                    let operator = filter
                        .get("type")
                        .and_then(|v| v.as_str())
                        .unwrap_or("equals")
                        .to_string();
                    if !variable.is_empty() {
                        conditions.push(Condition {
                            variable,
                            operator,
                            value,
                        });
                    }
                }
            }
        }
    }
    conditions
}

fn build_reverse_map(tags_data: &Value, exclude_tag_id: &str) -> HashMap<String, Vec<SharedTag>> {
    let mut map: HashMap<String, Vec<SharedTag>> = HashMap::new();
    let tags = tags_data
        .get("tag")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().collect::<Vec<_>>())
        .unwrap_or_default();

    for tag in tags {
        let tid = str_field(tag, "tagId");
        if tid == exclude_tag_id {
            continue;
        }
        let shared = SharedTag {
            id: tid,
            name: str_field(tag, "name"),
            tag_type: str_field(tag, "type"),
        };
        for key in ["firingTriggerId", "blockingTriggerId"] {
            if let Some(arr) = tag.get(key).and_then(|v| v.as_array()) {
                for trigger_id in arr.iter().filter_map(|v| v.as_str()) {
                    map.entry(trigger_id.to_string())
                        .or_default()
                        .push(SharedTag {
                            id: shared.id.clone(),
                            name: shared.name.clone(),
                            tag_type: shared.tag_type.clone(),
                        });
                }
            }
        }
    }
    map
}

fn str_field(value: &Value, key: &str) -> String {
    value
        .get(key)
        .and_then(|v| v.as_str())
        .unwrap_or("-")
        .to_string()
}

fn id_array(value: &Value, key: &str) -> Vec<String> {
    value
        .get(key)
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn extract_variable_refs(text: &str) -> Vec<String> {
    let mut vars = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let mut i = 0;
    let bytes = text.as_bytes();
    while i + 3 < bytes.len() {
        if bytes[i] == b'{' && bytes[i + 1] == b'{' {
            if let Some(end) = text[i + 2..].find("}}") {
                let name = &text[i + 2..i + 2 + end];
                if !name.starts_with('_') && !name.is_empty() && seen.insert(name.to_string()) {
                    vars.push(name.to_string());
                }
                i = i + 2 + end + 2;
                continue;
            }
        }
        i += 1;
    }
    vars
}

fn extract_key_params(tag: &Value) -> Vec<(String, String)> {
    let mut result = Vec::new();
    if let Some(params) = tag.get("parameter").and_then(|v| v.as_array()) {
        for p in params {
            let key = p.get("key").and_then(|v| v.as_str()).unwrap_or("");
            let value = p.get("value").and_then(|v| v.as_str()).unwrap_or("");
            match key {
                "html" => {
                    let preview = if value.len() > 80 {
                        format!("{}... ({} chars)", &value[..80], value.len())
                    } else {
                        value.to_string()
                    };
                    result.push((key.to_string(), preview));
                }
                "" => {}
                _ => {
                    result.push((key.to_string(), value.to_string()));
                }
            }
        }
    }
    result
}
