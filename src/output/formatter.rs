use clap::ValueEnum;
use serde_json::Value;

use super::table;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
    Compact,
}

pub fn print_output(value: &Value, format: &OutputFormat) {
    print_resource(value, format, "default");
}

pub fn print_resource(value: &Value, format: &OutputFormat, resource: &str) {
    match format {
        OutputFormat::Json => {
            // For JSON output, unwrap the API wrapper (e.g., {"tag": [...]} → [...])
            let output = unwrap_json(value);
            println!(
                "{}",
                serde_json::to_string_pretty(&output).unwrap_or_else(|_| output.to_string())
            );
        }
        OutputFormat::Table => {
            table::render(value, resource);
        }
        OutputFormat::Compact => {
            print_compact(value);
        }
    }
}

/// Unwrap GTM API list wrapper. If the value is an object whose only fields are
/// a single array and optionally "nextPageToken", return just the array.
/// Single resource objects and other shapes are returned as-is.
fn unwrap_json(value: &Value) -> Value {
    let Some(obj) = value.as_object() else {
        return value.clone();
    };
    let mut array_value = None;
    for (key, val) in obj {
        if key == "nextPageToken" {
            continue;
        }
        if val.is_array() && array_value.is_none() {
            array_value = Some(val);
        } else {
            // Multiple non-token fields or a non-array field -- don't unwrap
            return value.clone();
        }
    }
    array_value.cloned().unwrap_or_else(|| value.clone())
}

fn print_compact(value: &Value) {
    // ID field candidates in priority order
    let id_keys = [
        "accountId",
        "containerId",
        "workspaceId",
        "tagId",
        "triggerId",
        "variableId",
        "folderId",
        "templateId",
        "containerVersionId",
        "versionId",
        "environmentId",
        "clientId",
        "transformationId",
        "zoneId",
        "destinationId",
        "destinationLinkId",
        "permissionId",
        "gtagConfigId",
    ];

    let print_item = |item: &Value| {
        let id = id_keys
            .iter()
            .find_map(|k| item.get(k).and_then(|v| v.as_str()))
            .unwrap_or("-");
        let name = item.get("name").and_then(|v| v.as_str()).unwrap_or("-");
        println!("{id}\t{name}");
    };

    // Handle arrays at top level or wrapped in a resource key
    if let Some(arr) = value.as_array() {
        for item in arr {
            print_item(item);
        }
    } else if let Some(obj) = value.as_object() {
        // GTM API wraps lists in a resource key (e.g. {"tag": [...]})
        for (_key, val) in obj {
            if let Some(arr) = val.as_array() {
                for item in arr {
                    print_item(item);
                }
                return;
            }
        }
        // Single resource
        print_item(value);
    }
}

pub fn print_deleted(resource: &str, id: &str) {
    let msg = serde_json::json!({
        "status": "deleted",
        "resource": resource,
        "id": id,
    });
    println!("{}", serde_json::to_string_pretty(&msg).unwrap());
}
