use clap::{Args, Subcommand};
use serde_json::json;

use crate::api::client::GtmApiClient;
use crate::api::params::params_from_json;
use crate::api::workspace::resolve_workspace;
use crate::error::Result;
use crate::output::formatter::{print_output, OutputFormat};

#[derive(Args)]
pub struct SetupArgs {
    #[command(subcommand)]
    pub action: SetupAction,
}

#[derive(Args)]
struct WorkspaceFlags {
    #[arg(long, env = "GTM_ACCOUNT_ID")]
    account_id: String,
    #[arg(long, env = "GTM_CONTAINER_ID")]
    container_id: String,
    #[arg(long, env = "GTM_WORKSPACE_ID")]
    workspace_id: Option<String>,
}

#[derive(Subcommand)]
pub enum SetupAction {
    /// Complete GA4 setup (config tag + page view trigger + event tags)
    Ga4(SetupGa4Args),
    /// Facebook Pixel setup (pixel tag + page view trigger)
    FacebookPixel(SetupFacebookPixelArgs),
    /// Form submission tracking setup
    FormTracking(SetupFormTrackingArgs),
    /// Generate complete workflow by site type
    Workflow(SetupWorkflowArgs),
}

#[derive(Args)]
pub struct SetupGa4Args {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// GA4 Measurement ID (e.g., G-XXXXXXXXXX)
    #[arg(long)]
    measurement_id: String,
}

#[derive(Args)]
pub struct SetupFacebookPixelArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Facebook Pixel ID
    #[arg(long)]
    pixel_id: String,
}

#[derive(Args)]
pub struct SetupFormTrackingArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// CSS selector for the form
    #[arg(long)]
    form_selector: String,
    /// Custom event name
    #[arg(long, default_value = "form_submit")]
    event_name: String,
}

#[derive(Args)]
pub struct SetupWorkflowArgs {
    #[command(flatten)]
    ws: WorkspaceFlags,
    /// Workflow type: ecommerce, lead_generation, content_site
    #[arg(long = "type")]
    workflow_type: String,
    /// GA4 Measurement ID
    #[arg(long)]
    measurement_id: Option<String>,
    /// Facebook Pixel ID
    #[arg(long)]
    pixel_id: Option<String>,
}

async fn get_workspace_base(ws: &WorkspaceFlags, client: &GtmApiClient) -> Result<String> {
    let ws_id = resolve_workspace(
        client,
        &ws.account_id,
        &ws.container_id,
        ws.workspace_id.as_deref(),
    )
    .await?;
    Ok(format!(
        "accounts/{}/containers/{}/workspaces/{}",
        ws.account_id, ws.container_id, ws_id
    ))
}

async fn create_tag(
    client: &GtmApiClient,
    base: &str,
    name: &str,
    tag_type: &str,
    params: serde_json::Value,
) -> Result<serde_json::Value> {
    let parameters = params_from_json(&params);
    let body = json!({
        "name": name,
        "type": tag_type,
        "parameter": parameters,
    });
    client.post(&format!("{base}/tags"), &body).await
}

async fn create_trigger(
    client: &GtmApiClient,
    base: &str,
    name: &str,
    trigger_type: &str,
) -> Result<serde_json::Value> {
    let body = json!({
        "name": name,
        "type": trigger_type,
    });
    client.post(&format!("{base}/triggers"), &body).await
}

pub async fn handle(args: SetupArgs, client: &GtmApiClient, format: &OutputFormat) -> Result<()> {
    match args.action {
        SetupAction::Ga4(a) => {
            let base = get_workspace_base(&a.ws, client).await?;
            let mid = &a.measurement_id;
            let mut results = vec![];

            eprintln!("Setting up GA4 with measurement ID: {mid}");

            // GA4 Configuration Tag
            let config = create_tag(
                client,
                &base,
                "GA4 Configuration",
                "gaawc",
                json!({
                    "tagId": mid,
                    "measurementIdOverride": mid,
                }),
            )
            .await?;
            eprintln!("  Created: GA4 Configuration Tag");
            results.push(json!({"name": "GA4 Configuration Tag", "result": config}));

            // Page View Trigger
            let trigger = create_trigger(client, &base, "All Pages", "pageview").await?;
            eprintln!("  Created: All Pages Trigger");
            results.push(json!({"name": "All Pages Trigger", "result": trigger}));

            // Event tags
            let events = [
                (
                    "GA4 Event - Scroll Depth",
                    json!({"measurementIdOverride": mid, "eventName": "scroll", "scrollThreshold": "90"}),
                ),
                (
                    "GA4 Event - Outbound Click",
                    json!({"measurementIdOverride": mid, "eventName": "click", "clickType": "link"}),
                ),
                (
                    "GA4 Event - File Download",
                    json!({"measurementIdOverride": mid, "eventName": "file_download", "fileExtension": "pdf,doc,docx,xls,xlsx"}),
                ),
            ];

            for (name, params) in events {
                let tag = create_tag(client, &base, name, "gaawe", params).await?;
                eprintln!("  Created: {name}");
                results.push(json!({"name": name, "result": tag}));
            }

            let output = json!({
                "setup": "GA4 Complete Setup",
                "measurement_id": mid,
                "created": results.len(),
                "results": results,
            });
            print_output(&output, format);
        }

        SetupAction::FacebookPixel(a) => {
            let base = get_workspace_base(&a.ws, client).await?;
            let pid = &a.pixel_id;
            let mut results = vec![];

            eprintln!("Setting up Facebook Pixel: {pid}");

            let pixel_url = format!("https://www.facebook.com/tr?id={pid}&ev=PageView&noscript=1");
            let tag = create_tag(
                client,
                &base,
                "Facebook Pixel",
                "img",
                json!({
                    "url": pixel_url,
                }),
            )
            .await?;
            eprintln!("  Created: Facebook Pixel Tag");
            results.push(json!({"name": "Facebook Pixel Tag", "result": tag}));

            let trigger = create_trigger(client, &base, "All Pages - Facebook", "pageview").await?;
            eprintln!("  Created: All Pages Trigger");
            results.push(json!({"name": "All Pages Trigger", "result": trigger}));

            let output = json!({
                "setup": "Facebook Pixel Setup",
                "pixel_id": pid,
                "created": results.len(),
                "results": results,
            });
            print_output(&output, format);
        }

        SetupAction::FormTracking(a) => {
            let base = get_workspace_base(&a.ws, client).await?;
            let mut results = vec![];

            eprintln!("Setting up form tracking: {}", a.form_selector);

            // Form Submit Trigger
            let trigger_body = json!({
                "name": format!("Form Submit - {}", a.form_selector),
                "type": "formSubmission",
                "filter": [{
                    "type": "equals",
                    "parameter": [
                        {"key": "arg0", "value": "{{Form Element}}", "type": "template"},
                        {"key": "arg1", "value": a.form_selector, "type": "template"},
                    ]
                }]
            });
            let trigger = client
                .post(&format!("{base}/triggers"), &trigger_body)
                .await?;
            eprintln!("  Created: Form Submit Trigger");
            results.push(json!({"name": "Form Submit Trigger", "result": trigger}));

            let tag = create_tag(
                client,
                &base,
                &format!("Form Submit Event - {}", a.form_selector),
                "gaawe",
                json!({
                    "eventName": a.event_name,
                    "formSelector": a.form_selector,
                }),
            )
            .await?;
            eprintln!("  Created: Form Submit Event Tag");
            results.push(json!({"name": "Form Submit Event Tag", "result": tag}));

            let output = json!({
                "setup": "Form Tracking",
                "form_selector": a.form_selector,
                "event_name": a.event_name,
                "created": results.len(),
                "results": results,
            });
            print_output(&output, format);
        }

        SetupAction::Workflow(a) => {
            let base = get_workspace_base(&a.ws, client).await?;
            let mut results = vec![];

            eprintln!("Generating {} workflow...", a.workflow_type);

            match a.workflow_type.as_str() {
                "ecommerce" => {
                    if let Some(mid) = &a.measurement_id {
                        // GA4 base setup
                        let config = create_tag(
                            client,
                            &base,
                            "GA4 Configuration",
                            "gaawc",
                            json!({
                                "tagId": mid, "measurementIdOverride": mid,
                            }),
                        )
                        .await?;
                        results.push(json!({"name": "GA4 Config", "result": config}));

                        let trigger =
                            create_trigger(client, &base, "All Pages", "pageview").await?;
                        results.push(json!({"name": "All Pages", "result": trigger}));

                        // Ecommerce events
                        let events = [
                            (
                                "purchase",
                                json!({"measurementIdOverride": mid, "eventName": "purchase", "transactionId": "{{Transaction ID}}", "value": "{{Revenue}}"}),
                            ),
                            (
                                "add_to_cart",
                                json!({"measurementIdOverride": mid, "eventName": "add_to_cart", "itemId": "{{Item ID}}", "value": "{{Item Value}}"}),
                            ),
                            (
                                "remove_from_cart",
                                json!({"measurementIdOverride": mid, "eventName": "remove_from_cart", "itemId": "{{Item ID}}"}),
                            ),
                            (
                                "begin_checkout",
                                json!({"measurementIdOverride": mid, "eventName": "begin_checkout"}),
                            ),
                        ];
                        for (name, params) in events {
                            let tag = create_tag(
                                client,
                                &base,
                                &format!("GA4 Event - {name}"),
                                "gaawe",
                                params,
                            )
                            .await?;
                            eprintln!("  Created: GA4 Event - {name}");
                            results.push(
                                json!({"name": format!("GA4 Event - {name}"), "result": tag}),
                            );
                        }
                    }
                }
                "lead_generation" => {
                    if let Some(mid) = &a.measurement_id {
                        let config = create_tag(
                            client,
                            &base,
                            "GA4 Configuration",
                            "gaawc",
                            json!({
                                "tagId": mid, "measurementIdOverride": mid,
                            }),
                        )
                        .await?;
                        results.push(json!({"name": "GA4 Config", "result": config}));

                        let trigger =
                            create_trigger(client, &base, "All Pages", "pageview").await?;
                        results.push(json!({"name": "All Pages", "result": trigger}));

                        // CTA click
                        let cta = create_tag(client, &base, "CTA Click Event", "gaawe", json!({
                            "measurementIdOverride": mid, "eventName": "cta_click", "clickElement": "{{Click Element}}",
                        })).await?;
                        results.push(json!({"name": "CTA Click", "result": cta}));
                    }

                    // Form tracking
                    let form_trigger = json!({
                        "name": "Form Submit - #contact-form",
                        "type": "formSubmission",
                    });
                    let ft = client
                        .post(&format!("{base}/triggers"), &form_trigger)
                        .await?;
                    results.push(json!({"name": "Form Trigger", "result": ft}));

                    let form_tag = create_tag(
                        client,
                        &base,
                        "Form Submit Event",
                        "gaawe",
                        json!({
                            "eventName": "form_submit", "formSelector": "#contact-form",
                        }),
                    )
                    .await?;
                    results.push(json!({"name": "Form Event", "result": form_tag}));
                }
                "content_site" => {
                    if let Some(mid) = &a.measurement_id {
                        let config = create_tag(
                            client,
                            &base,
                            "GA4 Configuration",
                            "gaawc",
                            json!({
                                "tagId": mid, "measurementIdOverride": mid,
                            }),
                        )
                        .await?;
                        results.push(json!({"name": "GA4 Config", "result": config}));

                        let trigger =
                            create_trigger(client, &base, "All Pages", "pageview").await?;
                        results.push(json!({"name": "All Pages", "result": trigger}));

                        let events = [
                            (
                                "newsletter_signup",
                                json!({"measurementIdOverride": mid, "eventName": "newsletter_signup"}),
                            ),
                            (
                                "social_share",
                                json!({"measurementIdOverride": mid, "eventName": "social_share", "sharePlatform": "{{Share Platform}}"}),
                            ),
                            (
                                "video_play",
                                json!({"measurementIdOverride": mid, "eventName": "video_play", "videoTitle": "{{Video Title}}"}),
                            ),
                            (
                                "article_read",
                                json!({"measurementIdOverride": mid, "eventName": "article_read", "articleTitle": "{{Article Title}}"}),
                            ),
                        ];
                        for (name, params) in events {
                            let tag = create_tag(
                                client,
                                &base,
                                &format!("Content Event - {name}"),
                                "gaawe",
                                params,
                            )
                            .await?;
                            eprintln!("  Created: Content Event - {name}");
                            results.push(
                                json!({"name": format!("Content Event - {name}"), "result": tag}),
                            );
                        }
                    }
                }
                other => {
                    eprintln!("Unknown workflow type: {other}");
                    eprintln!("Available types: ecommerce, lead_generation, content_site");
                    return Ok(());
                }
            }

            // Facebook Pixel if provided
            if let Some(pid) = &a.pixel_id {
                let pixel_url =
                    format!("https://www.facebook.com/tr?id={pid}&ev=PageView&noscript=1");
                let fb_tag = create_tag(
                    client,
                    &base,
                    "Facebook Pixel",
                    "img",
                    json!({"url": pixel_url}),
                )
                .await?;
                results.push(json!({"name": "Facebook Pixel", "result": fb_tag}));

                let fb_trigger =
                    create_trigger(client, &base, "All Pages - Facebook", "pageview").await?;
                results.push(json!({"name": "FB Trigger", "result": fb_trigger}));
            }

            let output = json!({
                "workflow_type": a.workflow_type,
                "created": results.len(),
                "results": results,
            });
            print_output(&output, format);
        }
    }
    Ok(())
}
