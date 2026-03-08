//! Envelope Pattern for MCP Tool Responses
//!
//! Standardized response format with status, metadata, warnings, data/error.
//! Includes telemetry fields for observability.

use rmcp::model::{CallToolResult, Content};
use serde::Serialize;
use serde_json::{Value, json};
use std::time::Instant;
use uuid::Uuid;

/// Standardized warning codes for vector retrieval.
pub const WARNING_VECTOR_STORE_UNAVAILABLE: &str = "vector_store_unavailable";
pub const WARNING_EMBEDDER_TIMEOUT: &str = "embedder_timeout";
pub const WARNING_FALLBACK_TO_LEXICAL: &str = "fallback_to_lexical";

/// Response status indicating success, partial success, or error
#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum EnvelopeStatus {
    Ok,
    Partial,
    Error,
}

/// Cache hit level for telemetry
#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum CacheHit {
    /// L1 cache hit (in-memory)
    L1,
    /// L2 cache hit (disk-based)
    L2,
    /// No cache hit
    #[default]
    None,
}

impl CacheHit {
    pub fn as_str(&self) -> &'static str {
        match self {
            CacheHit::L1 => "l1",
            CacheHit::L2 => "l2",
            CacheHit::None => "none",
        }
    }
}

/// Enhanced metadata for envelope responses
#[derive(Debug, Serialize)]
pub struct EnvelopeMeta {
    /// Duration of the tool call in milliseconds
    pub duration_ms: u64,
    /// Cache hit level: "l1" | "l2" | "none"
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cache_hit: Option<String>,
    /// Whether this is a partial response
    pub partial_response: bool,
    /// Whether a timeout guard was triggered
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout_guard_triggered: Option<bool>,
    /// Number of rows/records scanned
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows_scanned: Option<usize>,
    /// Unique request ID for tracing
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
}

impl EnvelopeMeta {
    /// Create a new meta with duration calculated from start time
    pub fn from_start(started_at: Instant) -> Self {
        Self {
            duration_ms: started_at.elapsed().as_millis() as u64,
            cache_hit: None,
            partial_response: false,
            timeout_guard_triggered: None,
            rows_scanned: None,
            request_id: None,
        }
    }

    /// Set cache hit level
    pub fn with_cache_hit(mut self, cache_hit: CacheHit) -> Self {
        self.cache_hit = Some(cache_hit.as_str().to_string());
        self
    }

    /// Set partial response flag
    pub fn with_partial(mut self, partial: bool) -> Self {
        self.partial_response = partial;
        self
    }

    /// Set timeout guard triggered flag
    pub fn with_timeout_guard(mut self, triggered: bool) -> Self {
        self.timeout_guard_triggered = Some(triggered);
        self
    }

    /// Set rows scanned count
    pub fn with_rows_scanned(mut self, count: usize) -> Self {
        self.rows_scanned = Some(count);
        self
    }

    /// Set request ID
    pub fn with_request_id(mut self, id: impl Into<String>) -> Self {
        self.request_id = Some(id.into());
        self
    }
}

/// Error body for error responses
#[derive(Debug, Serialize)]
pub struct ErrorBody<'a> {
    /// Error code (e.g., "INVALID_ARGUMENT", "UNAVAILABLE")
    pub code: &'a str,
    /// Human-readable error message
    pub message: String,
    /// Optional additional details
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<Value>,
}

/// Builder for constructing envelope responses
pub struct EnvelopeBuilder {
    meta: EnvelopeMeta,
    warnings: Vec<String>,
}

impl EnvelopeBuilder {
    /// Create a new builder with start time
    pub fn new(started_at: Instant) -> Self {
        Self {
            meta: EnvelopeMeta::from_start(started_at),
            warnings: Vec::new(),
        }
    }

    /// Add a cache hit indicator
    pub fn cache_hit(mut self, hit: CacheHit) -> Self {
        self.meta.cache_hit = Some(hit.as_str().to_string());
        self
    }

    /// Set partial response flag
    pub fn partial(mut self, is_partial: bool) -> Self {
        self.meta.partial_response = is_partial;
        self
    }

    /// Set timeout guard triggered
    pub fn timeout_guard(mut self, triggered: bool) -> Self {
        self.meta.timeout_guard_triggered = Some(triggered);
        self
    }

    /// Set rows scanned count
    pub fn rows_scanned(mut self, count: usize) -> Self {
        self.meta.rows_scanned = Some(count);
        self
    }

    /// Set request ID
    pub fn request_id(mut self, id: impl Into<String>) -> Self {
        self.meta.request_id = Some(id.into());
        self
    }

    /// Add a warning message
    pub fn warning(mut self, warning: impl Into<String>) -> Self {
        self.warnings.push(warning.into());
        self
    }

    /// Add multiple warning messages
    pub fn warnings(mut self, warnings: Vec<String>) -> Self {
        self.warnings.extend(warnings);
        self
    }

    /// Build a success response with data
    pub fn success(self, data: Value) -> CallToolResult {
        let status = if self.meta.partial_response {
            EnvelopeStatus::Partial
        } else {
            EnvelopeStatus::Ok
        };
        let payload = json!({
            "status": status,
            "meta": self.meta,
            "warnings": self.warnings,
            "data": data
        });
        CallToolResult::success(vec![Content::text(payload.to_string())])
    }

    /// Build an error response
    pub fn error(
        self,
        code: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> CallToolResult {
        let payload = json!({
            "status": EnvelopeStatus::Error,
            "meta": self.meta,
            "warnings": self.warnings,
            "error": ErrorBody {
                code,
                message: message.into(),
                details
            }
        });
        CallToolResult::success(vec![Content::text(payload.to_string())])
    }
}

// =============================================================================
// Legacy compatibility functions (deprecated but kept for backward compatibility)
// =============================================================================

/// Legacy function: Check if a feature flag is enabled via environment variable.
/// Consider using `FeatureFlags` from `flags.rs` instead.
pub fn feature_flag_enabled(flag_name: &str, default_value: bool) -> bool {
    let env_key = format!(
        "CORTEX_FLAG_{}",
        flag_name
            .chars()
            .map(|c| {
                if c.is_ascii_alphanumeric() {
                    c.to_ascii_uppercase()
                } else {
                    '_'
                }
            })
            .collect::<String>()
    );
    match std::env::var(env_key) {
        Ok(v) => matches!(v.as_str(), "1" | "true" | "TRUE" | "yes" | "on"),
        Err(_) => default_value,
    }
}

/// Legacy function: Create a success envelope.
/// Consider using `EnvelopeBuilder` for more control over metadata.
pub fn success(
    data: Value,
    started_at: Instant,
    warnings: Vec<String>,
    partial: bool,
) -> CallToolResult {
    let status = if partial {
        EnvelopeStatus::Partial
    } else {
        EnvelopeStatus::Ok
    };
    let payload = json!({
        "status": status,
        "meta": {
            "duration_ms": started_at.elapsed().as_millis() as u64,
            "partial_response": partial
        },
        "warnings": warnings,
        "data": data
    });
    CallToolResult::success(vec![Content::text(payload.to_string())])
}

/// Legacy function: Create an error envelope.
/// Consider using `EnvelopeBuilder` for more control over metadata.
pub fn error(
    code: &'static str,
    message: impl Into<String>,
    details: Option<Value>,
    started_at: Instant,
) -> CallToolResult {
    let payload = json!({
        "status": EnvelopeStatus::Error,
        "meta": {
            "duration_ms": started_at.elapsed().as_millis() as u64,
            "partial_response": false
        },
        "warnings": [],
        "error": ErrorBody {
            code,
            message: message.into(),
            details
        }
    });
    CallToolResult::success(vec![Content::text(payload.to_string())])
}

// =============================================================================
// Helper functions
// =============================================================================

/// Generate a unique request ID
pub fn generate_request_id() -> String {
    Uuid::new_v4().to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use std::time::Instant;

    #[test]
    fn envelope_success_contains_status_and_data() {
        let out = success(json!({"x": 1}), Instant::now(), Vec::new(), false);
        let text = out.content[0].as_text().expect("text content").text.clone();
        assert!(text.contains("\"status\":\"ok\""));
        assert!(text.contains("\"x\":1"));
    }

    #[test]
    fn envelope_error_contains_code() {
        let out = error("INVALID_ARGUMENT", "bad input", None, Instant::now());
        let text = out.content[0].as_text().expect("text content").text.clone();
        assert!(text.contains("\"status\":\"error\""));
        assert!(text.contains("\"code\":\"INVALID_ARGUMENT\""));
    }

    #[test]
    fn feature_flag_reads_env() {
        assert!(!feature_flag_enabled(
            "mcp.unset_flag_for_test.enabled",
            false
        ));
    }

    #[test]
    fn envelope_builder_success() {
        let result = EnvelopeBuilder::new(Instant::now())
            .cache_hit(CacheHit::L1)
            .rows_scanned(42)
            .request_id("test-123")
            .success(json!({"result": "ok"}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["status"], "ok");
        assert_eq!(parsed["meta"]["cache_hit"], "l1");
        assert_eq!(parsed["meta"]["rows_scanned"], 42);
        assert_eq!(parsed["meta"]["request_id"], "test-123");
    }

    #[test]
    fn envelope_builder_partial() {
        let result = EnvelopeBuilder::new(Instant::now())
            .partial(true)
            .warning("fallback_relaxed")
            .success(json!({"items": []}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["status"], "partial");
        assert!(
            parsed["warnings"]
                .as_array()
                .unwrap()
                .contains(&json!("fallback_relaxed"))
        );
    }

    #[test]
    fn envelope_builder_error() {
        let result = EnvelopeBuilder::new(Instant::now())
            .timeout_guard(true)
            .error(
                "TIMEOUT",
                "operation timed out",
                Some(json!({"limit_ms": 5000})),
            );

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["status"], "error");
        assert_eq!(parsed["error"]["code"], "TIMEOUT");
        assert_eq!(parsed["meta"]["timeout_guard_triggered"], true);
    }

    #[test]
    fn cache_hit_as_str() {
        assert_eq!(CacheHit::L1.as_str(), "l1");
        assert_eq!(CacheHit::L2.as_str(), "l2");
        assert_eq!(CacheHit::None.as_str(), "none");
    }

    #[test]
    fn generate_request_id_returns_uuid() {
        let id = generate_request_id();
        assert!(uuid::Uuid::parse_str(&id).is_ok());
    }

    #[test]
    fn envelope_meta_from_start() {
        let meta = EnvelopeMeta::from_start(Instant::now());
        assert!(meta.cache_hit.is_none());
        assert!(!meta.partial_response);
        assert!(meta.timeout_guard_triggered.is_none());
        assert!(meta.rows_scanned.is_none());
        assert!(meta.request_id.is_none());
    }

    #[test]
    fn envelope_meta_with_methods() {
        let meta = EnvelopeMeta::from_start(Instant::now())
            .with_cache_hit(CacheHit::L2)
            .with_partial(true)
            .with_timeout_guard(true)
            .with_rows_scanned(100)
            .with_request_id("req-456");

        assert_eq!(meta.cache_hit, Some("l2".to_string()));
        assert!(meta.partial_response);
        assert_eq!(meta.timeout_guard_triggered, Some(true));
        assert_eq!(meta.rows_scanned, Some(100));
        assert_eq!(meta.request_id, Some("req-456".to_string()));
    }
}
