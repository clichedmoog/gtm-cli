use clap::ValueEnum;
use serde_json::Value;

use super::table;

#[derive(Debug, Clone, Copy, ValueEnum)]
pub enum OutputFormat {
    Json,
    Table,
}

pub fn print_output(value: &Value, format: &OutputFormat) {
    print_resource(value, format, "default");
}

pub fn print_resource(value: &Value, format: &OutputFormat, resource: &str) {
    match format {
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::to_string_pretty(value).unwrap_or_else(|_| value.to_string())
            );
        }
        OutputFormat::Table => {
            table::render(value, resource);
        }
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
