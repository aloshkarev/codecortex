//! # cortex-a2a
//!
//! Agent-to-agent (A2A) protocol layer for CodeCortex: internal envelopes, task store,
//! in-process bus, and JSON wire types aligned with A2A v1.0.

pub mod a2a_errors;
pub mod agent_card;
pub mod bus;
pub mod codec;
pub mod cooperation;
pub mod envelope;
pub mod hub;
pub mod manifest;
pub mod payload;
pub mod proto;
pub mod push;
pub mod roles;
pub mod runtime;
pub mod services;
pub mod session;
pub mod spec_codec;
pub mod task_events;
pub mod task_store;
pub mod wire;

pub use a2a_errors::{A2aErrorBody, A2aErrorKind};
pub use agent_card::{gateway_agent_card, role_agent_card};
pub use bus::A2aBus;
pub use codec::{envelope_to_message, message_to_envelope};
pub use cooperation::{
    CooperationArtifact, CooperationArtifactKind, EXTENSION_INTELLIGENCE_COOPERATION,
    task_cooperation_metadata,
};
pub use envelope::A2aEnvelope;
pub use hub::{A2aHub, SpawnSessionRequest, SpawnSessionResponse};
pub use manifest::{RoleManifest, RoleManifestRegistry};
pub use payload::{A2aPayload, RiskLevel};
pub use push::{PushDelivery, TaskPushNotificationConfig};
pub use roles::AgentRole;
pub use services::{
    A2aServices, ContextCapsuleSummary, ImpactSummary, IndexFreshnessLabel, IntelligenceRequest,
    NullA2aServices, PatchContextCapsule, SharedA2aServices, ValidationSummary,
    blackboard_from_envelope, spawn_tool_hints,
};
pub use session::{A2aTaskRecord, TaskState, TaskStore};
pub use spec_codec::{
    parse_send_request_json, send_request_proto_to_wire, task_record_to_proto, task_to_spec_json,
    task_wire_to_spec_json, task_wire_to_spec_json_with_options,
};
pub use task_events::TaskEventHub;
pub use wire::{
    A2aMessage, A2aPart, AgentCardWire, EXTENSION_BLACKBOARD, ListTasksResponseWire,
    SendMessageConfigurationWire, SendMessageRequestWire, SendMessageResponseWire,
    StreamResponseWire, TaskStateWire, TaskWire,
};
