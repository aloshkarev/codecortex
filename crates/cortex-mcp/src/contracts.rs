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
    NotModified,
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

/// Freshness status attached to agent-facing context responses.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FreshnessState {
    Fresh,
    Warming,
    Stale,
    Partial,
    Unknown,
}

/// Token budget accounting for bounded context tools.
#[derive(Debug, Clone, Serialize)]
pub struct TokenBudget {
    pub requested_tokens: usize,
    pub estimated_tokens: usize,
    pub hard_cap: bool,
}

/// Token savings metadata for bounded context tools.
#[derive(Debug, Clone, Serialize)]
pub struct TokenSavings {
    pub returned_tokens: usize,
    pub baseline_tokens: usize,
    pub saved_tokens: usize,
    pub baseline_estimated: bool,
    pub tokenizer: String,
}

/// Minimal source scope metadata for context provenance.
#[derive(Debug, Clone, Serialize)]
pub struct ResponseScope {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub repo_path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub include_paths: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub exclude_paths: Vec<String>,
}

/// Policy describing how much source material a response is allowed to expose.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SourcePolicy {
    MetadataOnly,
    Signatures,
    Snippets,
    FullSource,
    Forbidden,
}

fn source_policy_audit_label(p: &SourcePolicy) -> &'static str {
    match p {
        SourcePolicy::MetadataOnly => "metadata_only",
        SourcePolicy::Signatures => "signatures",
        SourcePolicy::Snippets => "snippets",
        SourcePolicy::FullSource => "full_source",
        SourcePolicy::Forbidden => "forbidden",
    }
}

/// Metadata for context omitted because of budget, policy, freshness, or scope.
#[derive(Debug, Clone, Serialize)]
pub struct OmittedItem {
    pub reason: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub symbol: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_tokens: Option<usize>,
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
    /// Cost class used by MCP clients to choose safe tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cost_class: Option<String>,
    /// Freshness state of indexed data used by the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub freshness: Option<FreshnessState>,
    /// Token budget accounting for context tools.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_budget: Option<TokenBudget>,
    /// Token savings vs naive full-source baseline.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub token_savings: Option<TokenSavings>,
    /// Source scope used for retrieval.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scope: Option<ResponseScope>,
    /// Source exposure policy applied to the response.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_policy: Option<SourcePolicy>,
    /// Omitted context details.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub omitted: Vec<OmittedItem>,
    /// Recommended next tools for progressive disclosure.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub next_tools: Vec<String>,
    /// Privacy or policy warnings.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub privacy_warnings: Vec<String>,
    /// Content fingerprint for conditional fetch (`if_none_match`).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub etag: Option<String>,
    /// Embedder tier used for vector retrieval (`static-fallback`, model name, etc.).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedder: Option<String>,
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
            cost_class: None,
            freshness: None,
            token_budget: None,
            token_savings: None,
            scope: None,
            source_policy: None,
            omitted: Vec::new(),
            next_tools: Vec::new(),
            privacy_warnings: Vec::new(),
            etag: None,
            embedder: None,
        }
    }

    /// Set content etag for conditional fetch.
    pub fn with_etag(mut self, etag: impl Into<String>) -> Self {
        self.etag = Some(etag.into());
        self
    }

    /// Set embedder label for vector responses.
    pub fn with_embedder(mut self, embedder: impl Into<String>) -> Self {
        self.embedder = Some(embedder.into());
        self
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

    /// Set MCP tool cost class.
    pub fn with_cost_class(mut self, class: impl Into<String>) -> Self {
        self.cost_class = Some(class.into());
        self
    }

    /// Set index freshness state.
    pub fn with_freshness(mut self, freshness: FreshnessState) -> Self {
        self.freshness = Some(freshness);
        self
    }

    /// Set token budget metadata.
    pub fn with_token_budget(mut self, budget: TokenBudget) -> Self {
        self.token_budget = Some(budget);
        self
    }

    /// Set token savings metadata.
    pub fn with_token_savings(mut self, savings: TokenSavings) -> Self {
        self.token_savings = Some(savings);
        self
    }

    /// Set response scope metadata.
    pub fn with_scope(mut self, scope: ResponseScope) -> Self {
        self.scope = Some(scope);
        self
    }

    /// Set source exposure policy.
    pub fn with_source_policy(mut self, policy: SourcePolicy) -> Self {
        self.source_policy = Some(policy);
        self
    }

    /// Set omitted context records.
    pub fn with_omitted(mut self, omitted: Vec<OmittedItem>) -> Self {
        self.omitted = omitted;
        self
    }

    /// Set recommended next tools.
    pub fn with_next_tools(mut self, next_tools: Vec<String>) -> Self {
        self.next_tools = next_tools;
        self
    }

    /// Set privacy warnings.
    pub fn with_privacy_warnings(mut self, privacy_warnings: Vec<String>) -> Self {
        self.privacy_warnings = privacy_warnings;
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
    started_at: Instant,
    meta: EnvelopeMeta,
    warnings: Vec<String>,
    /// When set, `success` / `error` emit a structured audit line and optional registry telemetry.
    audit_tool_name: Option<String>,
}

impl EnvelopeBuilder {
    /// Create a new builder with start time
    pub fn new(started_at: Instant) -> Self {
        Self {
            started_at,
            meta: EnvelopeMeta::from_start(started_at),
            warnings: Vec::new(),
            audit_tool_name: None,
        }
    }

    /// Tag this response for optional JSONL audit (`CORTEX_MCP_AUDIT_LOG`) and in-process telemetry.
    pub fn audit_tool(mut self, name: impl Into<String>) -> Self {
        self.audit_tool_name = Some(name.into());
        self
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

    /// Set MCP tool cost class metadata.
    pub fn cost_class(mut self, class: impl Into<String>) -> Self {
        self.meta.cost_class = Some(class.into());
        self
    }

    /// Set index freshness metadata.
    pub fn freshness(mut self, freshness: FreshnessState) -> Self {
        self.meta.freshness = Some(freshness);
        self
    }

    /// Set token budget metadata.
    pub fn token_budget(mut self, budget: TokenBudget) -> Self {
        self.meta.token_budget = Some(budget);
        self
    }

    /// Set token savings metadata.
    pub fn token_savings(mut self, savings: TokenSavings) -> Self {
        self.meta.token_savings = Some(savings);
        self
    }

    /// Set source scope metadata.
    pub fn scope(mut self, scope: ResponseScope) -> Self {
        self.meta.scope = Some(scope);
        self
    }

    /// Set source exposure policy.
    pub fn source_policy(mut self, policy: SourcePolicy) -> Self {
        self.meta.source_policy = Some(policy);
        self
    }

    /// Set omitted context records.
    pub fn omitted(mut self, omitted: Vec<OmittedItem>) -> Self {
        self.meta.omitted = omitted;
        self
    }

    /// Set recommended next tools.
    pub fn next_tools(mut self, next_tools: Vec<String>) -> Self {
        self.meta.next_tools = next_tools;
        self
    }

    /// Set privacy warnings.
    pub fn privacy_warnings(mut self, warnings: Vec<String>) -> Self {
        self.meta.privacy_warnings = warnings;
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

    /// Set content etag on envelope meta.
    pub fn etag(mut self, etag: impl Into<String>) -> Self {
        self.meta.etag = Some(etag.into());
        self
    }

    /// Set embedder label on envelope meta.
    pub fn embedder(mut self, label: impl Into<String>) -> Self {
        self.meta.embedder = Some(label.into());
        self
    }

    /// Build a not-modified response (conditional fetch hit).
    pub fn not_modified(mut self, etag: impl Into<String>) -> CallToolResult {
        self.meta.duration_ms = self.started_at.elapsed().as_millis() as u64;
        self.meta.etag = Some(etag.into());
        let payload = json!({
            "status": EnvelopeStatus::NotModified,
            "meta": self.meta,
            "warnings": self.warnings,
            "data": { "not_modified": true }
        });
        let payload_str = payload.to_string();
        let audit_name = self.audit_tool_name.clone();
        self.emit_audit_and_telemetry(audit_name.as_deref(), "not_modified", Some(payload_str.len()));
        CallToolResult::success(vec![Content::text(payload_str)])
    }

    /// Build a success response with data
    pub fn success(mut self, data: Value) -> CallToolResult {
        self.meta.duration_ms = self.started_at.elapsed().as_millis() as u64;
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
        let payload_str = payload.to_string();
        let plen = Some(payload_str.len());
        let audit_name = self.audit_tool_name.clone();
        self.emit_audit_and_telemetry(audit_name.as_deref(), "ok", plen);
        CallToolResult::success(vec![Content::text(payload_str)])
    }

    /// Build an error response
    pub fn error(
        mut self,
        code: &'static str,
        message: impl Into<String>,
        details: Option<Value>,
    ) -> CallToolResult {
        self.meta.duration_ms = self.started_at.elapsed().as_millis() as u64;
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
        let payload_str = payload.to_string();
        let plen = Some(payload_str.len());
        let audit_name = self.audit_tool_name.clone();
        self.emit_audit_and_telemetry(audit_name.as_deref(), "error", plen);
        CallToolResult::success(vec![Content::text(payload_str)])
    }

    fn emit_audit_and_telemetry(
        &self,
        tool: Option<&str>,
        status: &str,
        payload_chars: Option<usize>,
    ) {
        let Some(tool) = tool else {
            return;
        };
        let source_policy = self
            .meta
            .source_policy
            .as_ref()
            .map(source_policy_audit_label);
        let cost_class = self.meta.cost_class.clone();
        let repo_path = self.meta.scope.as_ref().and_then(|s| s.repo_path.clone());
        let duration_ms = self.started_at.elapsed().as_millis() as u64;
        crate::audit::log_tool_audit(crate::audit::ToolAuditEvent {
            ts_ms: crate::audit::now_ms(),
            tool: tool.to_string(),
            status: status.to_string(),
            duration_ms,
            payload_chars,
            source_policy: source_policy.map(|s| s.to_string()),
            cost_class,
            repo_path,
        });
        if crate::FeatureFlags::from_env().telemetry_enabled {
            let mut t = crate::telemetry::ToolTelemetry::new(tool.to_string());
            t.duration_ms = duration_ms;
            t.partial_response = self.meta.partial_response;
            if let Some(rp) = self.meta.scope.as_ref().and_then(|s| s.repo_path.clone()) {
                t.repo_path = Some(rp);
            }
            crate::telemetry::record_telemetry(t);
        }
    }
}


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
/// Serialize the standard success envelope as JSON text (for L1 cache bodies).
pub fn success_json(
    data: Value,
    started_at: Instant,
    warnings: Vec<String>,
    partial: bool,
) -> String {
    let status = if partial {
        EnvelopeStatus::Partial
    } else {
        EnvelopeStatus::Ok
    };
    json!({
        "status": status,
        "meta": {
            "duration_ms": started_at.elapsed().as_millis() as u64,
            "partial_response": partial
        },
        "warnings": warnings,
        "data": data
    })
    .to_string()
}

pub fn success(
    data: Value,
    started_at: Instant,
    warnings: Vec<String>,
    partial: bool,
) -> CallToolResult {
    CallToolResult::success(vec![Content::text(success_json(
        data, started_at, warnings, partial,
    ))])
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
    fn envelope_builder_not_modified() {
        let result = EnvelopeBuilder::new(Instant::now()).not_modified("etag-abc");
        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: serde_json::Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["status"], "not_modified");
        assert_eq!(parsed["meta"]["etag"], "etag-abc");
        assert_eq!(parsed["data"]["not_modified"], true);
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
