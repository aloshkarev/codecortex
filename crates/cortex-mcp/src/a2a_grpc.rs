//! gRPC binding for A2A v1 (`docs/a2a.proto`).

use cortex_a2a::A2aHub;
use cortex_a2a::wire::TaskStateWire;
use std::net::SocketAddr;
use std::pin::Pin;
use std::sync::Arc;
use tonic::transport::Server;
use tonic::{Request, Response, Status};

pub mod pb {
    tonic::include_proto!("lf.a2a.v1");
}

use pb::a2a_service_server::{A2aService, A2aServiceServer};
use pb::send_message_response::Payload as SendPayload;
use pb::stream_response::Payload as StreamPayload;
use pb::*;

pub struct A2aGrpcService {
    hub: Arc<A2aHub>,
}

impl A2aGrpcService {
    pub fn new(hub: Arc<A2aHub>) -> Self {
        Self { hub }
    }
}

fn wire_state_to_pb(state: TaskStateWire) -> i32 {
    let s = match state {
        TaskStateWire::TaskStateUnspecified => TaskState::Unspecified,
        TaskStateWire::TaskStateSubmitted => TaskState::Submitted,
        TaskStateWire::TaskStateWorking => TaskState::Working,
        TaskStateWire::TaskStateCompleted => TaskState::Completed,
        TaskStateWire::TaskStateFailed => TaskState::Failed,
        TaskStateWire::TaskStateCanceled => TaskState::Canceled,
        TaskStateWire::TaskStateInputRequired => TaskState::InputRequired,
        TaskStateWire::TaskStateRejected => TaskState::Rejected,
        TaskStateWire::TaskStateAuthRequired => TaskState::AuthRequired,
    };
    s as i32
}

fn task_to_pb(wire: cortex_a2a::TaskWire) -> Task {
    let artifacts = wire
        .artifacts
        .into_iter()
        .map(|a| Artifact {
            artifact_id: a.artifact_id,
            name: a.name.unwrap_or_default(),
            description: a.description.unwrap_or_default(),
            parts: a
                .parts
                .into_iter()
                .map(|p| Part {
                    metadata: None,
                    filename: String::new(),
                    media_type: p.media_type.unwrap_or_default(),
                    content: p.text.map(|t| part::Content::Text(t)),
                })
                .collect(),
            metadata: None,
            extensions: vec![],
        })
        .collect();

    Task {
        id: wire.id,
        context_id: wire.context_id.unwrap_or_default(),
        status: Some(TaskStatus {
            state: wire_state_to_pb(wire.status.state),
            message: wire.status.message.map(|m| wire_message_to_pb(m)),
            timestamp: None,
        }),
        artifacts,
        history: wire.history.into_iter().map(wire_message_to_pb).collect(),
        metadata: None,
    }
}

fn wire_message_to_pb(m: cortex_a2a::A2aMessage) -> Message {
    Message {
        message_id: m.message_id,
        context_id: m.context_id.unwrap_or_default(),
        task_id: m.task_id.unwrap_or_default(),
        role: if m.role == "user" {
            Role::User as i32
        } else {
            Role::Agent as i32
        },
        parts: m
            .parts
            .into_iter()
            .map(|p| Part {
                metadata: None,
                filename: String::new(),
                media_type: p.media_type.unwrap_or_default(),
                content: p.text.map(part::Content::Text),
            })
            .collect(),
        metadata: None,
        extensions: m.extensions,
        reference_task_ids: vec![],
    }
}

fn first_message_text(msg: &Message) -> String {
    msg.parts
        .first()
        .and_then(|p| match &p.content {
            Some(part::Content::Text(t)) => Some(t.clone()),
            _ => None,
        })
        .unwrap_or_default()
}

fn send_message_wire(req: SendMessageRequest) -> cortex_a2a::SendMessageRequestWire {
    let msg = req.message.unwrap_or(Message {
        message_id: uuid::Uuid::new_v4().to_string(),
        context_id: String::new(),
        task_id: String::new(),
        role: Role::User as i32,
        parts: vec![],
        metadata: None,
        extensions: vec![],
        reference_task_ids: vec![],
    });
    let text = first_message_text(&msg);
    let return_immediately = req
        .configuration
        .as_ref()
        .map(|c| c.return_immediately)
        .unwrap_or(false);

    cortex_a2a::SendMessageRequestWire {
        message: cortex_a2a::A2aMessage {
            message_id: if msg.message_id.is_empty() {
                uuid::Uuid::new_v4().to_string()
            } else {
                msg.message_id
            },
            context_id: if msg.context_id.is_empty() {
                None
            } else {
                Some(msg.context_id)
            },
            task_id: if msg.task_id.is_empty() {
                None
            } else {
                Some(msg.task_id)
            },
            role: "user".to_string(),
            parts: vec![cortex_a2a::A2aPart {
                text: Some(if text.is_empty() {
                    "A2A gRPC message".to_string()
                } else {
                    text
                }),
                data: None,
                metadata: None,
                media_type: None,
            }],
            metadata: None,
            extensions: msg.extensions,
        },
        configuration: Some(cortex_a2a::SendMessageConfigurationWire {
            return_immediately,
            history_length: req.configuration.and_then(|c| c.history_length),
        }),
    }
}

fn wire_agent_card_to_pb(wire: cortex_a2a::AgentCardWire, extension_uri: &str) -> AgentCard {
    AgentCard {
        name: wire.name,
        description: wire.description,
        version: wire.version,
        supported_interfaces: wire
            .supported_interfaces
            .into_iter()
            .map(|i| AgentInterface {
                url: i.url,
                protocol_binding: i.protocol_binding,
                protocol_version: i.protocol_version,
                tenant: i.tenant.unwrap_or_default(),
            })
            .collect(),
        capabilities: Some(AgentCapabilities {
            streaming: wire.capabilities.streaming,
            push_notifications: wire.capabilities.push_notifications,
            extended_agent_card: Some(true),
            extensions: vec![AgentExtension {
                uri: extension_uri.to_string(),
                description: "CodeCortex graph blackboard extension".to_string(),
                required: false,
                params: None,
            }],
        }),
        default_input_modes: wire.default_input_modes,
        default_output_modes: wire.default_output_modes,
        skills: wire
            .skills
            .into_iter()
            .map(|s| AgentSkill {
                id: s.id,
                name: s.name,
                description: s.description,
                tags: s.tags,
                examples: vec![],
                input_modes: vec![],
                output_modes: vec![],
                security_requirements: vec![],
            })
            .collect(),
        provider: None,
        documentation_url: None,
        security_schemes: Default::default(),
        security_requirements: vec![],
        signatures: vec![],
        icon_url: None,
    }
}

fn stream_wire_to_pb(event: cortex_a2a::StreamResponseWire) -> StreamResponse {
    if let Some(update) = event.status_update {
        return StreamResponse {
            payload: Some(StreamPayload::StatusUpdate(TaskStatusUpdateEvent {
                task_id: update.task_id,
                context_id: update.context_id,
                status: Some(TaskStatus {
                    state: wire_state_to_pb(update.status.state),
                    message: update.status.message.map(wire_message_to_pb),
                    timestamp: None,
                }),
                metadata: None,
            })),
        };
    }
    StreamResponse {
        payload: event.task.map(|t| StreamPayload::Task(task_to_pb(t))),
    }
}

#[tonic::async_trait]
impl A2aService for A2aGrpcService {
    async fn send_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<SendMessageResponse>, Status> {
        let wire_req = send_message_wire(request.into_inner());
        let resp = self
            .hub
            .send_message(wire_req)
            .map_err(|e| Status::internal(e.to_string()))?;
        let task = resp
            .task
            .map(task_to_pb)
            .ok_or_else(|| Status::internal("missing task"))?;
        Ok(Response::new(SendMessageResponse {
            payload: Some(SendPayload::Task(task)),
        }))
    }

    type SendStreamingMessageStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<StreamResponse, Status>> + Send>>;

    async fn send_streaming_message(
        &self,
        request: Request<SendMessageRequest>,
    ) -> Result<Response<Self::SendStreamingMessageStream>, Status> {
        let send = self.send_message(request).await?;
        let task = send
            .into_inner()
            .payload
            .and_then(|p| match p {
                SendPayload::Task(t) => Some(t),
                _ => None,
            })
            .ok_or_else(|| Status::internal("missing task"))?;
        let stream = async_stream::try_stream! {
            yield StreamResponse {
                payload: Some(StreamPayload::Task(task)),
            };
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn get_task(&self, request: Request<GetTaskRequest>) -> Result<Response<Task>, Status> {
        let id = request.into_inner().id;
        let wire = self
            .hub
            .get_task_wire(&id)
            .map_err(|e| Status::not_found(e.to_string()))?;
        Ok(Response::new(task_to_pb(wire)))
    }

    async fn list_tasks(
        &self,
        request: Request<ListTasksRequest>,
    ) -> Result<Response<ListTasksResponse>, Status> {
        let ctx = request.into_inner().context_id;
        let ctx_opt = if ctx.is_empty() {
            None
        } else {
            Some(ctx.as_str())
        };
        let list = self
            .hub
            .list_tasks_wire(ctx_opt)
            .map_err(|e| Status::internal(e.to_string()))?;
        let n = list.tasks.len() as i32;
        Ok(Response::new(ListTasksResponse {
            tasks: list.tasks.into_iter().map(task_to_pb).collect(),
            next_page_token: String::new(),
            page_size: n,
            total_size: n,
        }))
    }

    async fn cancel_task(
        &self,
        request: Request<CancelTaskRequest>,
    ) -> Result<Response<Task>, Status> {
        let id = request.into_inner().id;
        let wire = self
            .hub
            .cancel_task(&id)
            .map_err(|e| Status::not_found(e.to_string()))?;
        Ok(Response::new(task_to_pb(wire)))
    }

    type SubscribeToTaskStream =
        Pin<Box<dyn tokio_stream::Stream<Item = Result<StreamResponse, Status>> + Send>>;

    async fn subscribe_to_task(
        &self,
        request: Request<SubscribeToTaskRequest>,
    ) -> Result<Response<Self::SubscribeToTaskStream>, Status> {
        let id = request.into_inner().id;
        let task_id =
            uuid::Uuid::parse_str(&id).map_err(|e| Status::invalid_argument(e.to_string()))?;
        let mut rx = self.hub.subscribe_task(&task_id);
        let hub = self.hub.clone();
        let stream = async_stream::try_stream! {
            if let Ok(wire) = hub.get_task_wire(&id) {
                yield stream_wire_to_pb(cortex_a2a::StreamResponseWire {
                    task: Some(wire),
                    status_update: None,
                    artifact_update: None,
                });
            }
            loop {
                match rx.recv().await {
                    Ok(event) => {
                        let terminal = event.task.as_ref().is_some_and(|t| {
                            matches!(
                                t.status.state,
                                TaskStateWire::TaskStateCompleted
                                    | TaskStateWire::TaskStateFailed
                                    | TaskStateWire::TaskStateCanceled
                            )
                        });
                        yield stream_wire_to_pb(event);
                        if terminal {
                            break;
                        }
                    }
                    Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                    Err(_) => break,
                }
            }
        };
        Ok(Response::new(Box::pin(stream)))
    }

    async fn create_task_push_notification_config(
        &self,
        request: Request<TaskPushNotificationConfig>,
    ) -> Result<Response<TaskPushNotificationConfig>, Status> {
        if !self.hub.config.push.enabled {
            return Err(Status::failed_precondition(
                "push notifications disabled — set [a2a.push].enabled = true",
            ));
        }
        let req = request.into_inner();
        let saved = self
            .hub
            .push()
            .create_config(cortex_a2a::TaskPushNotificationConfig {
                id: if req.id.is_empty() {
                    uuid::Uuid::new_v4().to_string()
                } else {
                    req.id.clone()
                },
                task_id: req.task_id.clone(),
                url: req.url.clone(),
                token: if req.token.is_empty() {
                    None
                } else {
                    Some(req.token.clone())
                },
            });
        Ok(Response::new(TaskPushNotificationConfig {
            tenant: String::new(),
            id: saved.id,
            task_id: saved.task_id,
            url: saved.url,
            token: saved.token.unwrap_or_default(),
            authentication: None,
        }))
    }

    async fn get_task_push_notification_config(
        &self,
        request: Request<GetTaskPushNotificationConfigRequest>,
    ) -> Result<Response<TaskPushNotificationConfig>, Status> {
        let req = request.into_inner();
        let cfg = self
            .hub
            .push()
            .get_config(&req.id)
            .filter(|c| c.task_id == req.task_id)
            .ok_or_else(|| Status::not_found("push config not found"))?;
        Ok(Response::new(TaskPushNotificationConfig {
            tenant: String::new(),
            id: cfg.id,
            task_id: cfg.task_id,
            url: cfg.url,
            token: cfg.token.unwrap_or_default(),
            authentication: None,
        }))
    }

    async fn list_task_push_notification_configs(
        &self,
        request: Request<ListTaskPushNotificationConfigsRequest>,
    ) -> Result<Response<ListTaskPushNotificationConfigsResponse>, Status> {
        let task_id = request.into_inner().task_id;
        let configs = self
            .hub
            .push()
            .list_for_task(&task_id)
            .into_iter()
            .map(|c| TaskPushNotificationConfig {
                tenant: String::new(),
                id: c.id,
                task_id: c.task_id,
                url: c.url,
                token: c.token.unwrap_or_default(),
                authentication: None,
            })
            .collect();
        Ok(Response::new(ListTaskPushNotificationConfigsResponse {
            configs,
            next_page_token: String::new(),
        }))
    }

    async fn get_extended_agent_card(
        &self,
        _request: Request<GetExtendedAgentCardRequest>,
    ) -> Result<Response<AgentCard>, Status> {
        let listen = self.hub.config.server.grpc_listen.clone();
        let wire = cortex_a2a::gateway_agent_card(&self.hub.config, &format!("http://{listen}"));
        Ok(Response::new(wire_agent_card_to_pb(
            wire,
            &self.hub.config.server.extension_uri,
        )))
    }

    async fn delete_task_push_notification_config(
        &self,
        request: Request<DeleteTaskPushNotificationConfigRequest>,
    ) -> Result<Response<()>, Status> {
        let req = request.into_inner();
        if !self.hub.push().delete_config(&req.id) {
            return Err(Status::not_found("push config not found"));
        }
        Ok(Response::new(()))
    }
}

pub async fn serve_grpc(addr: SocketAddr, hub: Arc<A2aHub>) -> anyhow::Result<()> {
    let svc = A2aGrpcService::new(hub);
    Server::builder()
        .add_service(A2aServiceServer::new(svc))
        .serve(addr)
        .await?;
    Ok(())
}
