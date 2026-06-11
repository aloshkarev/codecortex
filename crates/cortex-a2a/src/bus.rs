use crate::envelope::A2aEnvelope;
use crate::roles::AgentRole;
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};

const BUS_CAPACITY: usize = 1024;

/// In-process message bus for agent envelopes.
#[derive(Clone)]
pub struct A2aBus {
    tx: broadcast::Sender<Arc<A2aEnvelope>>,
    role_inboxes: Arc<dashmap::DashMap<AgentRole, mpsc::Sender<Arc<A2aEnvelope>>>>,
}

impl A2aBus {
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(BUS_CAPACITY);
        Self {
            tx,
            role_inboxes: Arc::new(dashmap::DashMap::new()),
        }
    }

    pub fn register_role(
        &self,
        role: AgentRole,
        buffer: usize,
    ) -> mpsc::Receiver<Arc<A2aEnvelope>> {
        let (role_tx, role_rx) = mpsc::channel(buffer.max(16));
        self.role_inboxes.insert(role, role_tx);
        role_rx
    }

    pub async fn publish(&self, envelope: A2aEnvelope) {
        let arc = Arc::new(envelope);
        let _ = self.tx.send(arc.clone());
        if let Some(inbox) = self.role_inboxes.get(&arc.receiver) {
            let _ = inbox.send(arc).await;
        }
    }

    pub fn subscribe_all(&self) -> broadcast::Receiver<Arc<A2aEnvelope>> {
        self.tx.subscribe()
    }
}

impl Default for A2aBus {
    fn default() -> Self {
        Self::new()
    }
}
