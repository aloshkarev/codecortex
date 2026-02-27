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

// =============================================================================
// Schema Validation Tests
// =============================================================================

mod schema_validation {
    use super::*;

    fn validate_envelope(json: &Value) -> Result<(), String> {
        let schema: Value = serde_json::from_str(ENVELOPE_SCHEMA).map_err(|e| e.to_string())?;
        let compiled = jsonschema::JSONSchema::compile(&schema).map_err(|e| e.to_string())?;
        let result = compiled.validate(json);
        if let Err(errors) = result {
            let error_messages: Vec<String> = errors
                .map(|e: jsonschema::ValidationError| e.to_string())
                .collect();
            return Err(format!("Validation errors: {}", error_messages.join(", ")));
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

        // Verify all fields are present
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

// =============================================================================
// Error Code Conformance Tests
// =============================================================================

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

// =============================================================================
// Meta Field Tests
// =============================================================================

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
        // Duration should be very small for instant completion
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

// =============================================================================
// Status Tests
// =============================================================================

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

// =============================================================================
// Feature Flag Tests
// =============================================================================

mod feature_flags {
    use cortex_mcp::FeatureFlags;

    #[test]
    fn feature_flags_global_is_consistent() {
        let flags1 = FeatureFlags::global();
        let flags2 = FeatureFlags::global();

        // Both should return the same static reference
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

        // Both naming conventions should work
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

// =============================================================================
// Integration Tests - Response Content
// =============================================================================

mod integration {
    use super::*;

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

        // Should parse as valid JSON
        let parsed: Result<Value, _> = serde_json::from_str(&text);
        assert!(parsed.is_ok(), "Response should be valid JSON");
    }
}
