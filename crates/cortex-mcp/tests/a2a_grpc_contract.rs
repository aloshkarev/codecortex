//! gRPC contract: SendMessage through `A2aGrpcService` (normative `docs/a2a.proto`).

use cortex_a2a::A2aHub;
use cortex_core::A2aConfig;
use cortex_mcp::a2a_grpc::A2aGrpcService;
use cortex_mcp::a2a_grpc::pb::a2a_service_server::A2aService;
use cortex_mcp::a2a_grpc::pb::send_message_response::Payload;
use cortex_mcp::a2a_grpc::pb::{
    Message, Part, Role, SendMessageConfiguration, SendMessageRequest, part,
};
use std::sync::Arc;
use tonic::Request;

#[tokio::test]
async fn grpc_send_message_returns_task() {
    let hub = Arc::new(A2aHub::new(A2aConfig {
        enabled: true,
        ..A2aConfig::default()
    }));
    let svc = A2aGrpcService::new(hub);

    let resp = svc
        .send_message(Request::new(SendMessageRequest {
            tenant: String::new(),
            message: Some(Message {
                message_id: uuid::Uuid::new_v4().to_string(),
                context_id: String::new(),
                task_id: String::new(),
                role: Role::User as i32,
                parts: vec![Part {
                    metadata: None,
                    filename: String::new(),
                    media_type: String::new(),
                    content: Some(part::Content::Text("Review transport deadlock".to_string())),
                }],
                metadata: None,
                extensions: vec![],
                reference_task_ids: vec![],
            }),
            configuration: Some(SendMessageConfiguration {
                accepted_output_modes: vec![],
                task_push_notification_config: None,
                history_length: None,
                return_immediately: true,
            }),
            metadata: None,
        }))
        .await
        .expect("send_message")
        .into_inner();

    let task = match resp.payload {
        Some(Payload::Task(t)) => t,
        _ => panic!("expected task payload"),
    };
    assert!(!task.id.is_empty());
}
