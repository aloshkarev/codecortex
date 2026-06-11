//! In-process role runners and gateway dispatch.

mod context;
mod external;
mod gateway;
mod runners;
mod supervisor;
mod workflow;

pub use external::send_and_collect_replies;

pub use context::RoleContext;
pub use gateway::RoleGateway;
pub use runners::{RoleRunner, build_runners};
pub use supervisor::BusSupervisor;
pub use workflow::WorkflowEngine;
