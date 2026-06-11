//! A2A blackboard nodes (`AgentInsight`, `A2aSession`) on the code graph.

use crate::GraphClient;
use chrono::{DateTime, Utc};
use cortex_core::Result;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

/// Agent insight posted to the shared graph blackboard.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInsightRecord {
    pub id: String,
    pub session_id: String,
    pub conversation_id: String,
    pub role: String,
    pub summary: String,
    pub target_qualified_name: String,
    pub risk_level: String,
    pub suggested_action: String,
    pub created_at: DateTime<Utc>,
}

/// Compute stable insight id for idempotent writes.
pub fn insight_id(session_id: &str, role: &str, target: &str, summary: &str) -> String {
    let mut h = DefaultHasher::new();
    session_id.hash(&mut h);
    role.hash(&mut h);
    target.hash(&mut h);
    summary.hash(&mut h);
    format!("insight:{:016x}", h.finish())
}

pub struct BlackboardWriter {
    client: GraphClient,
    batch_size: usize,
}

impl BlackboardWriter {
    pub fn new(client: GraphClient, batch_size: usize) -> Self {
        Self {
            client,
            batch_size: batch_size.max(1),
        }
    }

    pub async fn ensure_schema(&self) -> Result<()> {
        crate::schema::ensure_a2a_schema(&self.client).await
    }

    pub async fn upsert_session(
        &self,
        session_id: &str,
        conversation_id: &str,
        state: &str,
    ) -> Result<()> {
        let q = r#"
            MERGE (s:A2aSession {id: $id})
            SET s.conversation_id = $conversation_id,
                s.state = $state,
                s.updated_at = $updated_at
        "#;
        self.client
            .query_with_params(
                q,
                vec![
                    ("id", session_id.to_string()),
                    ("conversation_id", conversation_id.to_string()),
                    ("state", state.to_string()),
                    ("updated_at", Utc::now().to_rfc3339()),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn write_insight(&self, insight: &AgentInsightRecord) -> Result<()> {
        let q = r#"
            MERGE (i:AgentInsight {id: $id})
            SET i.session_id = $session_id,
                i.conversation_id = $conversation_id,
                i.role = $role,
                i.summary = $summary,
                i.target_qualified_name = $target,
                i.risk_level = $risk_level,
                i.suggested_action = $suggested_action,
                i.created_at = $created_at
            WITH i
            OPTIONAL MATCH (c:CodeNode {qualified_name: $target})
            FOREACH (_ IN CASE WHEN c IS NULL THEN [] ELSE [1] END |
                MERGE (c)-[:HAS_INSIGHT]->(i)
            )
        "#;
        self.client
            .query_with_params(
                q,
                vec![
                    ("id", insight.id.clone()),
                    ("session_id", insight.session_id.clone()),
                    ("conversation_id", insight.conversation_id.clone()),
                    ("role", insight.role.clone()),
                    ("summary", insight.summary.clone()),
                    ("target", insight.target_qualified_name.clone()),
                    ("risk_level", insight.risk_level.clone()),
                    ("suggested_action", insight.suggested_action.clone()),
                    ("created_at", insight.created_at.to_rfc3339()),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn write_insights_batch(&self, insights: &[AgentInsightRecord]) -> Result<()> {
        for chunk in insights.chunks(self.batch_size) {
            for insight in chunk {
                self.write_insight(insight).await?;
            }
        }
        Ok(())
    }

    /// Planner/indexer hint linking a file path to a session (MUTATION_HINT).
    pub async fn write_mutation_hint(
        &self,
        session_id: &str,
        conversation_id: &str,
        file_path: &str,
        event_type: &str,
    ) -> Result<()> {
        let q = r#"
            MERGE (s:A2aSession {id: $session_id})
            SET s.conversation_id = $conversation_id,
                s.updated_at = $updated_at
            WITH s
            OPTIONAL MATCH (c:CodeNode {path: $path})
            FOREACH (_ IN CASE WHEN c IS NULL THEN [] ELSE [1] END |
                MERGE (s)-[:MUTATION_HINT {event_type: $event_type}]->(c)
            )
        "#;
        self.client
            .query_with_params(
                q,
                vec![
                    ("session_id", session_id.to_string()),
                    ("conversation_id", conversation_id.to_string()),
                    ("path", file_path.to_string()),
                    ("event_type", event_type.to_string()),
                    ("updated_at", Utc::now().to_rfc3339()),
                ],
            )
            .await?;
        Ok(())
    }

    pub async fn list_insights(&self, session_id: &str) -> Result<Vec<AgentInsightRecord>> {
        let rows = self
            .client
            .query_with_params(
                r#"
                MATCH (i:AgentInsight {session_id: $sid})
                RETURN i.id AS id, i.session_id AS session_id, i.conversation_id AS conversation_id,
                       i.role AS role, i.summary AS summary, i.target_qualified_name AS target,
                       i.risk_level AS risk_level, i.suggested_action AS suggested_action,
                       i.created_at AS created_at
                ORDER BY i.created_at DESC
                LIMIT 500
                "#,
                vec![("sid", session_id.to_string())],
            )
            .await?;
        let mut out = Vec::new();
        for row in rows {
            let created_at = row
                .get("created_at")
                .and_then(|v| v.as_str())
                .and_then(|s| DateTime::parse_from_rfc3339(s).ok())
                .map(|dt| dt.with_timezone(&Utc))
                .unwrap_or_else(Utc::now);
            out.push(AgentInsightRecord {
                id: row
                    .get("id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                session_id: row
                    .get("session_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                conversation_id: row
                    .get("conversation_id")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                role: row
                    .get("role")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                summary: row
                    .get("summary")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                target_qualified_name: row
                    .get("target")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                risk_level: row
                    .get("risk_level")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                suggested_action: row
                    .get("suggested_action")
                    .and_then(|v| v.as_str())
                    .unwrap_or_default()
                    .to_string(),
                created_at,
            });
        }
        Ok(out)
    }

    /// Remove insights older than `ttl_secs` for a session; returns deleted count.
    pub async fn prune_session_insights(&self, session_id: &str, ttl_secs: u64) -> Result<usize> {
        if ttl_secs == 0 {
            return Ok(0);
        }
        let cutoff = (Utc::now() - chrono::Duration::seconds(ttl_secs as i64)).to_rfc3339();
        let rows = self
            .client
            .query_with_params(
                r#"
                MATCH (i:AgentInsight {session_id: $sid})
                WHERE i.created_at < $cutoff
                WITH i, i.id AS iid
                DETACH DELETE i
                RETURN count(iid) AS deleted
                "#,
                vec![("sid", session_id.to_string()), ("cutoff", cutoff)],
            )
            .await?;
        Ok(rows
            .first()
            .and_then(|r| r.get("deleted"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize)
    }

    pub async fn count_insights(&self, session_id: &str) -> Result<usize> {
        let rows = self
            .client
            .query_with_params(
                "MATCH (i:AgentInsight {session_id: $sid}) RETURN count(i) AS c",
                vec![("sid", session_id.to_string())],
            )
            .await?;
        Ok(rows
            .first()
            .and_then(|r| r.get("c"))
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize)
    }
}
