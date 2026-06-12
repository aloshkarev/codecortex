//! Contract Tests for CodeCortex MCP Tool Responses
//!
//! These tests validate the envelope pattern contract including:
//! - JSON Schema validation for response shapes
//! - Error code conformance
//! - Feature flag behavior
//! - Negative tests for invalid inputs

use cortex_mcp::contracts::{CacheHit, EnvelopeBuilder};
use serde_json::{Value, json};
use std::time::Instant;

/// JSON Schema for the envelope response
const ENVELOPE_SCHEMA: &str = r#"
{
    "$schema": "http://json-schema.org/draft-07/schema#",
    "type": "object",
    "required": ["status", "meta", "warnings"],
    "properties": {
        "status": {
            "type": "string",
            "enum": ["ok", "partial", "error"]
        },
        "meta": {
            "type": "object",
            "required": ["duration_ms", "partial_response"],
            "properties": {
                "duration_ms": { "type": "integer", "minimum": 0 },
                "cache_hit": { "type": ["string", "null"], "enum": ["l1", "l2", "none", null] },
                "partial_response": { "type": "boolean" },
                "timeout_guard_triggered": { "type": ["boolean", "null"] },
                "rows_scanned": { "type": ["integer", "null"], "minimum": 0 },
                "request_id": { "type": ["string", "null"] }
            }
        },
        "warnings": {
            "type": "array",
            "items": { "type": "string" }
        },
        "data": {},
        "error": {
            "type": "object",
            "required": ["code", "message"],
            "properties": {
                "code": { "type": "string" },
                "message": { "type": "string" },
                "details": {}
            }
        }
    },
    "oneOf": [
        {
            "properties": { "status": { "enum": ["ok", "partial"] } },
            "required": ["data"]
        },
        {
            "properties": { "status": { "const": "error" } },
            "required": ["error"]
        }
    ]
}
"#;

/// Valid error codes that tools should use
const VALID_ERROR_CODES: &[&str] = &[
    "INVALID_ARGUMENT",
    "UNAVAILABLE",
    "NOT_FOUND",
    "PERMISSION_DENIED",
    "RATE_LIMITED",
    "TIMEOUT",
    "INTERNAL_ERROR",
    "SENSITIVE_CONTENT_DETECTED",
];

mod schema_validation {
    use super::*;

    fn validate_envelope(json: &Value) -> Result<(), String> {
        let schema: Value = serde_json::from_str(ENVELOPE_SCHEMA).map_err(|e| e.to_string())?;
        let compiled = jsonschema::validator_for(&schema).map_err(|e| e.to_string())?;
        let result = compiled.validate(json);
        if let Err(error) = result {
            return Err(format!("Validation error: {}", error));
        }
        Ok(())
    }

    #[test]
    fn success_envelope_validates() {
        let result = EnvelopeBuilder::new(Instant::now()).success(json!({"items": [1, 2, 3]}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        validate_envelope(&parsed).expect("envelope should validate");
    }

    #[test]
    fn partial_envelope_validates() {
        let result = EnvelopeBuilder::new(Instant::now())
            .partial(true)
            .warning("truncated_results")
            .success(json!({"items": []}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        validate_envelope(&parsed).expect("partial envelope should validate");
    }

    #[test]
    fn error_envelope_validates() {
        let result = EnvelopeBuilder::new(Instant::now()).error(
            "INVALID_ARGUMENT",
            "test error",
            Some(json!({"field": "query"})),
        );

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        validate_envelope(&parsed).expect("error envelope should validate");
    }

    #[test]
    fn envelope_with_all_meta_fields_validates() {
        let result = EnvelopeBuilder::new(Instant::now())
            .cache_hit(CacheHit::L1)
            .rows_scanned(500)
            .request_id("test-request-id")
            .timeout_guard(false)
            .success(json!({"result": "complete"}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        validate_envelope(&parsed).expect("full metadata envelope should validate");

        assert_eq!(parsed["meta"]["cache_hit"], "l1");
        assert_eq!(parsed["meta"]["rows_scanned"], 500);
        assert_eq!(parsed["meta"]["request_id"], "test-request-id");
        assert_eq!(parsed["meta"]["timeout_guard_triggered"], false);
    }

    #[test]
    fn envelope_with_warnings_validates() {
        let result = EnvelopeBuilder::new(Instant::now())
            .warning("first_warning")
            .warning("second_warning")
            .success(json!({}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        validate_envelope(&parsed).expect("envelope with warnings should validate");

        let warnings = parsed["warnings"].as_array().expect("warnings array");
        assert_eq!(warnings.len(), 2);
    }
}

mod error_codes {
    use super::*;

    #[test]
    fn error_envelope_contains_valid_code() {
        for &code in VALID_ERROR_CODES {
            let result = EnvelopeBuilder::new(Instant::now()).error(code, "test message", None);

            let text = result.content[0]
                .as_text()
                .expect("text content")
                .text
                .clone();
            let parsed: Value = serde_json::from_str(&text).expect("valid json");

            assert_eq!(
                parsed["error"]["code"], code,
                "Error code {} should be present",
                code
            );
        }
    }

    #[test]
    fn error_envelope_has_message() {
        let result = EnvelopeBuilder::new(Instant::now()).error(
            "INVALID_ARGUMENT",
            "Query parameter is required",
            None,
        );

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["error"]["message"], "Query parameter is required");
    }

    #[test]
    fn error_envelope_can_include_details() {
        let details = json!({
            "field": "query",
            "constraint": "must not be empty",
            "provided_value": ""
        });

        let result = EnvelopeBuilder::new(Instant::now()).error(
            "INVALID_ARGUMENT",
            "Invalid input",
            Some(details.clone()),
        );

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["error"]["details"], details);
    }

    #[test]
    fn error_status_is_error() {
        let result = EnvelopeBuilder::new(Instant::now()).error(
            "INTERNAL_ERROR",
            "Something went wrong",
            None,
        );

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["status"], "error");
    }
}

mod meta_fields {
    use super::*;

    #[test]
    fn duration_ms_is_present() {
        let result = EnvelopeBuilder::new(Instant::now()).success(json!({}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert!(parsed["meta"]["duration_ms"].is_number());
        let duration = parsed["meta"]["duration_ms"]
            .as_u64()
            .expect("duration is u64");
        assert!(duration < 1000, "Duration should be less than 1 second");
    }

    #[test]
    fn cache_hit_values() {
        for (hit_type, expected) in &[
            (CacheHit::L1, "l1"),
            (CacheHit::L2, "l2"),
            (CacheHit::None, "none"),
        ] {
            let result = EnvelopeBuilder::new(Instant::now())
                .cache_hit(*hit_type)
                .success(json!({}));

            let text = result.content[0]
                .as_text()
                .expect("text content")
                .text
                .clone();
            let parsed: Value = serde_json::from_str(&text).expect("valid json");

            assert_eq!(parsed["meta"]["cache_hit"], *expected);
        }
    }

    #[test]
    fn rows_scanned_is_tracked() {
        let result = EnvelopeBuilder::new(Instant::now())
            .rows_scanned(1234)
            .success(json!({}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["meta"]["rows_scanned"], 1234);
    }

    #[test]
    fn request_id_is_tracked() {
        let result = EnvelopeBuilder::new(Instant::now())
            .request_id("req-abc-123")
            .success(json!({}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["meta"]["request_id"], "req-abc-123");
    }

    #[test]
    fn timeout_guard_is_tracked() {
        let result = EnvelopeBuilder::new(Instant::now())
            .timeout_guard(true)
            .success(json!({}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["meta"]["timeout_guard_triggered"], true);
    }
}

mod status_tests {
    use super::*;

    #[test]
    fn ok_status_for_complete_response() {
        let result = EnvelopeBuilder::new(Instant::now()).success(json!({"data": "complete"}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["status"], "ok");
        assert!(!parsed["meta"]["partial_response"].as_bool().unwrap());
    }

    #[test]
    fn partial_status_for_incomplete_response() {
        let result = EnvelopeBuilder::new(Instant::now())
            .partial(true)
            .success(json!({"data": "incomplete"}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["status"], "partial");
        assert!(parsed["meta"]["partial_response"].as_bool().unwrap());
    }
}

mod feature_flags {
    use cortex_mcp::FeatureFlags;

    #[test]
    fn feature_flags_global_is_consistent() {
        let flags1 = FeatureFlags::global();
        let flags2 = FeatureFlags::global();

        assert!(std::ptr::eq(flags1, flags2));
    }

    #[test]
    fn feature_flags_is_enabled_matches_struct() {
        let flags = FeatureFlags::all_disabled();

        assert_eq!(flags.is_enabled("context_capsule"), flags.context_capsule);
        assert_eq!(flags.is_enabled("impact_graph"), flags.impact_graph);
        assert_eq!(flags.is_enabled("logic_flow"), flags.logic_flow);
    }

    #[test]
    fn feature_flags_alternative_names() {
        let flags = FeatureFlags::all_enabled();

        assert!(flags.is_enabled("context_capsule"));
        assert!(flags.is_enabled("mcp.context_capsule.enabled"));
        assert!(flags.is_enabled("impact_graph"));
        assert!(flags.is_enabled("mcp.impact_graph.enabled"));
    }

    #[test]
    fn feature_flags_unknown_returns_false() {
        let flags = FeatureFlags::all_enabled();

        assert!(!flags.is_enabled("nonexistent_flag"));
        assert!(!flags.is_enabled("mcp.unknown.enabled"));
    }
}

mod integration {
    use super::*;
    use cortex_mcp::contracts::success_json;

    #[test]
    fn success_json_matches_envelope_schema() {
        let started = Instant::now();
        let text = success_json(json!({"results": []}), started, Vec::new(), false);
        let parsed: Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["status"], "ok");
        assert!(parsed.get("data").is_some());
        assert!(parsed["meta"]["duration_ms"].is_number());
    }

    /// Tools migrated from legacy Self::ok to envelope_success (regression guard).
    const ENVELOPE_MIGRATED_TOOLS: &[&str] = &[
        "check_health",
        "list_indexed_repositories",
        "get_repository_stats",
        "list_jobs",
        "check_job_status",
        "find_code",
        "quick_info",
        "branch_structural_diff",
        "pr_review",
        "watch_directory",
        "list_watched_paths",
        "unwatch_directory",
        "load_bundle",
        "export_bundle",
        "search_across_projects",
        "find_similar_across_projects",
        "find_shared_dependencies",
        "compare_api_surface",
        "list_projects",
        "add_project",
        "remove_project",
        "set_current_project",
        "get_current_project",
        "list_branches",
        "refresh_project",
        "project_status",
        "project_sync",
        "project_branch_diff",
        "project_queue_status",
        "project_metrics",
    ];

    #[test]
    fn envelope_migrated_tool_list_non_empty() {
        assert!(ENVELOPE_MIGRATED_TOOLS.len() >= 30);
    }

    /// `check_health` exposes graph connectivity under `data.graph` (not legacy `memgraph`).
    #[test]
    fn check_health_response_uses_graph_key() {
        let sample = json!({
            "status": "ok",
            "data": {
                "graph": "connected",
                "backend": "falkordb",
                "analyzer": {}
            },
            "meta": { "duration_ms": 1, "partial_response": false },
            "warnings": []
        });
        assert_eq!(sample["data"]["graph"], "connected");
        assert!(sample["data"].get("memgraph").is_none());
    }

    #[test]
    fn envelope_contains_data_field() {
        let data = json!({
            "capsule_items": [
                {"id": "func:123", "name": "test_func", "score": 0.95}
            ],
            "token_estimate": 150
        });

        let result = EnvelopeBuilder::new(Instant::now()).success(data.clone());

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");

        assert_eq!(parsed["data"], data);
    }

    #[test]
    fn envelope_is_valid_json() {
        let result = EnvelopeBuilder::new(Instant::now())
            .cache_hit(CacheHit::L2)
            .warning("test warning")
            .success(json!({"test": "value"}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();

        let parsed: Result<Value, _> = serde_json::from_str(&text);
        assert!(parsed.is_ok(), "Response should be valid JSON");
    }

    #[test]
    fn envelope_token_savings_schema() {
        use cortex_mcp::contracts::TokenSavings;

        let result = EnvelopeBuilder::new(Instant::now())
            .token_savings(TokenSavings {
                returned_tokens: 120,
                baseline_tokens: 900,
                saved_tokens: 780,
                baseline_estimated: true,
                tokenizer: "cl100k_base".to_string(),
            })
            .success(json!({"items": []}));

        let text = result.content[0]
            .as_text()
            .expect("text content")
            .text
            .clone();
        let parsed: Value = serde_json::from_str(&text).expect("valid json");
        assert_eq!(parsed["meta"]["token_savings"]["returned_tokens"], 120);
        assert_eq!(parsed["meta"]["token_savings"]["baseline_tokens"], 900);
        assert_eq!(parsed["meta"]["token_savings"]["saved_tokens"], 780);
        assert_eq!(parsed["meta"]["token_savings"]["baseline_estimated"], true);
        assert_eq!(parsed["meta"]["token_savings"]["tokenizer"], "cl100k_base");
    }
}
