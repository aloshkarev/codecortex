use crate::GraphClient;
use cortex_core::{CodeEdge, CodeNode, Result};

/// Writes graph nodes and edges to the database.
///
/// Each `write_nodes` / `write_edges` call groups its input into chunks of
/// `batch_size` and issues a single bulk-upsert Cypher query per chunk (using
/// `UNWIND` on the Memgraph backend).  This reduces network round-trips from
/// O(N) individual queries to O(N / batch_size), which is critical when the
/// graph database is remote.
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
        let chunk_size = self.batch_size.max(1);
        for chunk in nodes.chunks(chunk_size) {
            self.client.bulk_upsert_nodes(chunk).await?;
        }
        Ok(())
    }

    pub async fn write_edges(&self, edges: &[CodeEdge]) -> Result<()> {
        let chunk_size = self.batch_size.max(1);
        for chunk in edges.chunks(chunk_size) {
            self.client.bulk_upsert_edges(chunk).await?;
        }
        Ok(())
    }
}
