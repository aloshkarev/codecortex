//! Per-task broadcast streams for SSE/gRPC subscribers.

use crate::wire::StreamResponseWire;
use dashmap::DashMap;
use std::sync::Arc;
use tokio::sync::broadcast;
use uuid::Uuid;

const STREAM_CAPACITY: usize = 64;

#[derive(Clone, Default)]
pub struct TaskEventHub {
    channels: Arc<DashMap<Uuid, broadcast::Sender<StreamResponseWire>>>,
}

impl TaskEventHub {
    pub fn subscribe(&self, task_id: &Uuid) -> broadcast::Receiver<StreamResponseWire> {
        self.channel_for(task_id).subscribe()
    }

    pub fn publish(&self, task_id: &Uuid, event: StreamResponseWire) {
        let _ = self.channel_for(task_id).send(event);
    }

    fn channel_for(&self, task_id: &Uuid) -> broadcast::Sender<StreamResponseWire> {
        if let Some(tx) = self.channels.get(task_id) {
            return tx.clone();
        }
        let (tx, _) = broadcast::channel(STREAM_CAPACITY);
        self.channels.insert(*task_id, tx.clone());
        tx
    }
}
