#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use std::sync::Arc;
use std::time::{Duration, SystemTime};
use tokio::sync::RwLock;
use tokio::time::sleep;
use tracing::{debug, error, info, warn};
use uuid::Uuid;

use frostgate_icap::chainadapter::{ChainAdapter, AdapterError};
use frostgate_sdk::message::{FrostMessage, MessageEvent, ChainId};

use crate::queue::MessageQueue;
use crate::types::{MessageStatus, QueuedMessage, RelayerConfig};
use crate::error::RelayerError;

type DynChainAdapter = dyn ChainAdapter<
    Error = AdapterError,
    BlockId = String,
    TxId = String
> + Send + Sync;

/// Main relayer service that handles cross-chain message propagation
pub struct RelayerService {
    /// Message processing queue
    queue: Arc<MessageQueue>,
    
    /// Configuration
    config: RelayerConfig,
    
    /// Source chain adapter
    source_chain: Arc<DynChainAdapter>,
    
    /// Destination chain adapter
    dest_chain: Arc<DynChainAdapter>,
    
    /// Whether the service is running
    running: Arc<RwLock<bool>>,
}

impl RelayerService {
    /// Create a new relayer service
    pub fn new(
        config: RelayerConfig,
        source_chain: Arc<DynChainAdapter>,
        dest_chain: Arc<DynChainAdapter>,
    ) -> Self {
        Self {
            queue: Arc::new(MessageQueue::new()),
            config,
            source_chain,
            dest_chain,
            running: Arc::new(RwLock::new(false)),
        }
    }

    /// Start the relayer service
    pub async fn start(&self) -> Result<(), RelayerError> {
        info!("Starting relayer service");
        
        let mut running = self.running.write().await;
        if *running {
            return Err(RelayerError::Configuration("Service already running".into()));
        }
        *running = true;
        drop(running);

        // Spawn message processing task
        let queue = self.queue.clone();
        let config = self.config.clone();
        let source_chain = self.source_chain.clone();
        let dest_chain = self.dest_chain.clone();
        let running = self.running.clone();

        tokio::spawn(async move {
            while *running.read().await {
                if let Err(e) = Self::process_messages(
                    &queue,
                    &config,
                    &source_chain,
                    &dest_chain,
                ).await {
                    error!("Error processing messages: {}", e);
                }
                sleep(Duration::from_secs(1)).await;
            }
        });

        // Spawn chain watching task
        let queue = self.queue.clone();
        let running = self.running.clone();
        let source_chain = self.source_chain.clone();

        tokio::spawn(async move {
            while *running.read().await {
                if let Err(e) = Self::watch_chain(&queue, &source_chain).await {
                    error!("Error watching chain: {}", e);
                }
                sleep(Duration::from_secs(1)).await;
            }
        });

        Ok(())
    }

    /// Stop the relayer service
    pub async fn stop(&self) {
        info!("Stopping relayer service");
        let mut running = self.running.write().await;
        *running = false;
    }

    /// Watch the source chain for new messages
    async fn watch_chain(
        queue: &MessageQueue,
        source_chain: &Arc<DynChainAdapter>,
    ) -> Result<(), RelayerError> {
        debug!("Watching chain for new messages");
        
        // Get new events from the chain
        let events = source_chain.listen_for_events().await
            .map_err(RelayerError::ChainAdapter)?;

        // Process each event
        for event in events {
            if let Err(e) = Self::handle_chain_event(queue, event).await {
                error!("Error handling chain event: {}", e);
            }
        }

        Ok(())
    }

    /// Handle a new chain event
    async fn handle_chain_event(
        queue: &MessageQueue,
        event: MessageEvent,
    ) -> Result<(), RelayerError> {
        debug!("Handling chain event: {:?}", event);

        // Validate the message
        Self::validate_message(&event.message)?;

        // Create queued message
        let queued = QueuedMessage {
            id: Uuid::new_v4(),
            message: event.message,
            status: MessageStatus::Pending,
            queued_at: SystemTime::now(),
            attempts: 0,
            last_error: None,
        };

        // Add to queue
        queue.enqueue(queued).await;

        Ok(())
    }

    /// Process messages in the queue
    async fn process_messages(
        queue: &MessageQueue,
        config: &RelayerConfig,
        source_chain: &Arc<DynChainAdapter>,
        dest_chain: &Arc<DynChainAdapter>,
    ) -> Result<(), RelayerError> {
        // Get next message to process
        if let Some(msg) = queue.dequeue().await {
            debug!("Processing message: {}", msg.id);

            // Check if we should retry
            if msg.attempts >= config.max_retry_attempts {
                queue.update_status(
                    msg.id,
                    MessageStatus::Failed("Max retry attempts exceeded".into()),
                    None,
                ).await;
                return Ok(());
            }

            // Verify message proof if present
            if let Some(proof) = msg.message.proof.as_ref() {
                if let Err(e) = source_chain.verify_proof(&msg.message).await {
                    queue.update_status(
                        msg.id,
                        MessageStatus::Failed("Proof verification failed".into()),
                        Some(e.to_string()),
                    ).await;
                    return Ok(());
                }
            }

            // Mark as ready for submission
            queue.update_status(msg.id, MessageStatus::ReadyForSubmission, None).await;

            // Submit to destination chain
            match dest_chain.submit_message(&msg.message).await {
                Ok(_) => {
                    queue.update_status(msg.id, MessageStatus::Submitted, None).await;
                }
                Err(e) => {
                    queue.update_status(
                        msg.id,
                        MessageStatus::Failed("Destination chain submission failed".into()),
                        Some(e.to_string()),
                    ).await;
                }
            }
        }

        // Prune old messages if enabled
        if config.enable_auto_pruning {
            queue.prune_old_messages(config.message_history_hours).await;
        }

        Ok(())
    }

    /// Validate a message
    fn validate_message(message: &FrostMessage) -> Result<(), RelayerError> {
        // TODO: Implement proper message validation
        // For now, just do basic checks
        
        if message.from_chain == ChainId::Unknown {
            return Err(RelayerError::ValidationError("Source chain is unknown".into()));
        }

        if message.to_chain == ChainId::Unknown {
            return Err(RelayerError::ValidationError("Target chain is unknown".into()));
        }

        if message.payload.is_empty() {
            return Err(RelayerError::ValidationError("Payload is empty".into()));
        }

        Ok(())
    }
} 