use crate::{EdgeWriteProfile, GraphClient};
use cortex_core::{CodeEdge, CodeNode, Result};
use tokio::task::JoinSet;

/// Writes graph entities in configured batches.
///
/// On Memgraph, each chunk is persisted with `UNWIND` bulk upserts. On backends
/// without a bulk implementation yet, the graph client keeps a compatible
/// per-entity fallback.
#[derive(Clone)]
pub struct NodeWriter {
    client: GraphClient,
    batch_size: usize,
}

impl NodeWriter {
    pub fn new(client: GraphClient, batch_size: usize) -> Self {
        Self { client, batch_size }
    }

    pub async fn write_nodes(&self, nodes: &[CodeNode]) -> Result<()> {
        self.write_nodes_concurrent(nodes, 1).await
    }

    /// Bulk-upsert nodes in chunks; `max_in_flight` concurrent chunk writes (FalkorDB pool-safe).
    pub async fn write_nodes_concurrent(
        &self,
        nodes: &[CodeNode],
        max_in_flight: usize,
    ) -> Result<()> {
        let chunk_size = self.batch_size.max(1);
        let chunks: Vec<&[CodeNode]> = nodes.chunks(chunk_size).collect();
        if chunks.is_empty() {
            return Ok(());
        }
        let parallel = max_in_flight.max(1).min(chunks.len());
        if parallel == 1 {
            for chunk in chunks {
                self.client.bulk_upsert_nodes(chunk).await?;
            }
            return Ok(());
        }
        let client = self.client.clone();
        for wave in chunks.chunks(parallel) {
            let mut join = JoinSet::new();
            for chunk in wave {
                let client = client.clone();
                let owned: Vec<CodeNode> = chunk.to_vec();
                join.spawn(async move { client.bulk_upsert_nodes(&owned).await });
            }
            while let Some(res) = join.join_next().await {
                res.map_err(|e| {
                    cortex_core::CortexError::Database(format!("node chunk join: {e}"))
                })??;
            }
        }
        Ok(())
    }

    /// Returns total Bolt write executions for this batch (see [`GraphClient::bulk_upsert_edges`]).
    pub async fn write_edges(&self, edges: &[CodeEdge]) -> Result<u64> {
        self.write_edges_profiled(edges, None).await
    }

    /// Like [`Self::write_edges`] with optional per-relationship-type profiling.
    pub async fn write_edges_profiled(
        &self,
        edges: &[CodeEdge],
        mut profile: Option<&mut EdgeWriteProfile>,
    ) -> Result<u64> {
        let chunk_size = self.batch_size.max(1);
        let mut bolt = 0u64;
        for chunk in edges.chunks(chunk_size) {
            bolt += self
                .client
                .bulk_upsert_edges_profiled(chunk, profile.as_deref_mut())
                .await?;
        }
        Ok(bolt)
    }
}
