//! A2A spec error types (§3.3.2 / §5.4) for HTTP+JSON bindings.

use serde::Serialize;
use serde_json::Value;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "PascalCase")]
pub enum A2aErrorKind {
    TaskNotFoundError,
    TaskNotCancelableError,
    PushNotificationNotSupportedError,
    UnsupportedOperationError,
    ContentTypeNotSupportedError,
    InvalidAgentResponseError,
    ExtendedAgentCardNotConfiguredError,
    ExtensionSupportRequiredError,
    VersionNotSupportedError,
}

#[derive(Debug, Clone, Serialize)]
pub struct A2aErrorBody {
    pub error: A2aErrorKind,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
}

impl A2aErrorBody {
    pub fn new(kind: A2aErrorKind, message: impl Into<String>) -> Self {
        Self {
            error: kind,
            message: Some(message.into()),
        }
    }

    /// HTTP status code per spec §5.4.
    pub fn status_code(&self) -> u16 {
        match self.error {
            A2aErrorKind::TaskNotFoundError => 404,
            A2aErrorKind::TaskNotCancelableError
            | A2aErrorKind::PushNotificationNotSupportedError
            | A2aErrorKind::UnsupportedOperationError
            | A2aErrorKind::ContentTypeNotSupportedError
            | A2aErrorKind::ExtendedAgentCardNotConfiguredError
            | A2aErrorKind::ExtensionSupportRequiredError
            | A2aErrorKind::VersionNotSupportedError => 400,
            A2aErrorKind::InvalidAgentResponseError => 500,
        }
    }

    pub fn to_json(&self) -> Value {
        serde_json::to_value(self)
            .unwrap_or_else(|_| serde_json::json!({"error": "InvalidAgentResponseError"}))
    }

    /// Axum response with spec error JSON body.
    pub fn into_http_response(self) -> (u16, Value) {
        (self.status_code(), self.to_json())
    }
}
