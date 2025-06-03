#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;
use crate::types::{QueuedMessage, MessageStatus};

/// Thread-safe message queue for handling cross-chain messages
pub struct MessageQueue {
    /// Queue of message IDs in order of processing
    pending: Arc<RwLock<VecDeque<Uuid>>>,
    /// Map of all messages by ID
    messages: Arc<RwLock<HashMap<Uuid, QueuedMessage>>>,
}

impl MessageQueue {
    /// Create a new message queue
    pub fn new() -> Self {
        Self {
            pending: Arc::new(RwLock::new(VecDeque::new())),
            messages: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// Add a message to the queue
    pub async fn enqueue(&self, message: QueuedMessage) {
        let mut pending = self.pending.write().await;
        let mut messages = self.messages.write().await;
        
        pending.push_back(message.id);
        messages.insert(message.id, message);
    }

    /// Get the next message to process
    pub async fn dequeue(&self) -> Option<QueuedMessage> {
        let mut pending = self.pending.write().await;
        let messages = self.messages.read().await;

        while let Some(id) = pending.pop_front() {
            if let Some(msg) = messages.get(&id) {
                if msg.status == MessageStatus::Pending {
                    return Some(msg.clone());
                }
            }
        }
        None
    }

    /// Update the status of a message
    pub async fn update_status(&self, id: Uuid, status: MessageStatus, error: Option<String>) {
        let mut messages = self.messages.write().await;
        if let Some(msg) = messages.get_mut(&id) {
            let is_failed = matches!(status, MessageStatus::Failed(_));
            msg.status = status;
            msg.last_error = error;
            if is_failed {
                msg.attempts += 1;
            }
        }
    }

    /// Get a message by ID
    pub async fn get_message(&self, id: &Uuid) -> Option<QueuedMessage> {
        let messages = self.messages.read().await;
        messages.get(id).cloned()
    }

    /// Remove processed messages older than the specified duration
    pub async fn prune_old_messages(&self, max_age_hours: u64) {
        let now = std::time::SystemTime::now();
        let mut messages = self.messages.write().await;
        let mut pending = self.pending.write().await;

        messages.retain(|&id, msg| {
            let age = now.duration_since(msg.queued_at).unwrap_or_default();
            let should_keep = age.as_secs() < max_age_hours * 3600 || 
                            matches!(msg.status, MessageStatus::Pending | MessageStatus::Proving);
            if !should_keep {
                pending.retain(|&x| x != id);
            }
            should_keep
        });
    }
} 