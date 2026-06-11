//! Reference external A2A client for CodeCortex role agents (patch planner, validator).
//!
//! Configure endpoints in `~/.cortex/config.toml` under `[a2a.roles.*]`.

use cortex_a2a::{A2aMessage, A2aPart, SendMessageConfigurationWire, SendMessageRequestWire};
use cortex_core::CortexConfig;
use serde_json::json;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let config = CortexConfig::load()?;
    let listen = &config.mcp.network.listen;
    let base = format!("http://{listen}");
    let mut args = std::env::args().skip(1);
    let subscribe = args.any(|a| a == "--subscribe");
    let args: Vec<_> = args.filter(|a| a != "--subscribe").collect();
    let role = args
        .first()
        .cloned()
        .unwrap_or_else(|| "patch_planner".to_string());
    let task = args
        .get(1)
        .cloned()
        .unwrap_or_else(|| "Review patch proposal".to_string());

    let client = reqwest::Client::new();

    if subscribe {
        let send_url = format!("{base}/a2a/v1/message:send");
        let req = SendMessageRequestWire {
            message: A2aMessage {
                message_id: uuid::Uuid::new_v4().to_string(),
                context_id: None,
                task_id: None,
                role: "user".to_string(),
                parts: vec![A2aPart {
                    text: Some(task.clone()),
                    data: Some(json!({"role": role})),
                    metadata: None,
                    media_type: Some("text/plain".to_string()),
                }],
                metadata: None,
                extensions: vec![cortex_a2a::EXTENSION_BLACKBOARD.to_string()],
            },
            configuration: Some(SendMessageConfigurationWire {
                return_immediately: true,
                history_length: None,
            }),
        };
        let res = client.post(&send_url).json(&req).send().await?;
        let body: serde_json::Value = res.json().await?;
        let task_id = body
            .get("task")
            .and_then(|t| t.get("id"))
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow::anyhow!("missing task id in send response"))?;
        let sub_url = format!("{base}/a2a/v1/tasks/{task_id}/subscribe");
        let response = client.get(&sub_url).send().await?;
        let body = response.text().await?;
        println!("{body}");
        return Ok(());
    }

    let url = format!("{base}/a2a/v1/message:send");
    let req = SendMessageRequestWire {
        message: A2aMessage {
            message_id: uuid::Uuid::new_v4().to_string(),
            context_id: None,
            task_id: None,
            role: "user".to_string(),
            parts: vec![A2aPart {
                text: Some(task),
                data: Some(json!({"role": role})),
                metadata: None,
                media_type: Some("text/plain".to_string()),
            }],
            metadata: None,
            extensions: vec![cortex_a2a::EXTENSION_BLACKBOARD.to_string()],
        },
        configuration: Some(SendMessageConfigurationWire {
            return_immediately: true,
            history_length: None,
        }),
    };

    let res = client.post(&url).json(&req).send().await?;
    let body = res.text().await?;
    println!("{body}");
    Ok(())
}
