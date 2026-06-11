//! Host-side guards to keep MCP responses bounded when A2A is enabled.

/// Truncate Cypher result rows for the host context; returns `(rows, was_truncated)`.
pub fn truncate_cypher_rows(
    mut rows: Vec<serde_json::Value>,
    max_rows: usize,
) -> (Vec<serde_json::Value>, bool) {
    if max_rows == 0 {
        return (rows, false);
    }
    let truncated = rows.len() > max_rows;
    if truncated {
        rows.truncate(max_rows);
    }
    (rows, truncated)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn truncates_when_over_max() {
        let rows: Vec<_> = (0..10).map(|i| json!({"n": i})).collect();
        let (out, truncated) = truncate_cypher_rows(rows, 3);
        assert!(truncated);
        assert_eq!(out.len(), 3);
        assert_eq!(out[2]["n"], 2);
    }

    #[test]
    fn no_truncate_when_under_max() {
        let rows = vec![json!({"n": 1})];
        let (out, truncated) = truncate_cypher_rows(rows, 50);
        assert!(!truncated);
        assert_eq!(out.len(), 1);
    }
}
