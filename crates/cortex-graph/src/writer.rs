use crate::GraphClient;
use cortex_core::{CodeEdge, CodeNode, Result};

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
        for chunk in nodes.chunks(self.batch_size.max(1)) {
            for node in chunk {
                self.client.upsert_node(node).await?;
            }
        }
        Ok(())
    }

    pub async fn write_edges(&self, edges: &[CodeEdge]) -> Result<()> {
        for chunk in edges.chunks(self.batch_size.max(1)) {
            for edge in chunk {
                self.client.upsert_edge(edge).await?;
            }
        }
        Ok(())
    }
}
