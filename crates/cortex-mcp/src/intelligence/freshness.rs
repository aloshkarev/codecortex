//! Path-scoped index freshness from branch metadata.

use cortex_core::IndexFreshness;
use cortex_graph::{GraphClient, get_branch_indexes};

/// Freshness label for scoped paths: `fresh`, `stale`, `partial`, or `unknown`.
pub async fn path_freshness(
    client: &GraphClient,
    repo_path: &str,
    include_paths: &[String],
) -> anyhow::Result<String> {
    let records = get_branch_indexes(client, repo_path).await?;
    let Some(record) = records.first() else {
        return Ok(IndexFreshness::Unknown.as_str().to_string());
    };

    if include_paths.is_empty() {
        return Ok(record.graph_freshness.as_str().to_string());
    }

    // When scoped, degrade to stale/partial if repo-wide freshness is not fresh.
    let base = record.graph_freshness.as_str().to_string();
    if base != "fresh" {
        return Ok(base);
    }

    // Scoped paths with fresh repo index: still fresh (path-level watermark not tracked yet).
    Ok("fresh".to_string())
}
