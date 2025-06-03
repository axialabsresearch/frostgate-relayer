#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_mut)]
#![allow(unused_must_use)]
#![allow(dead_code)]

use frostgate_sdk::frostmessage::FrostMessage;
use std::time::SystemTime;
use uuid::Uuid;

/// Status of a message being relayed
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MessageStatus {
    /// Message has been received and is pending processing
    Pending,
    /// Message is being processed by the prover
    Proving,
    /// Message has been proven and is ready for submission
    ReadyForSubmission,
    /// Message has been submitted to destination chain
    Submitted,
    /// Message has been finalized on destination chain
    Finalized,
    /// Message processing failed
    Failed(String),
}

/// A message in the relay queue
#[derive(Debug, Clone)]
pub struct QueuedMessage {
    /// Unique identifier for this queued message
    pub id: Uuid,
    /// The actual message to be relayed
    pub message: FrostMessage,
    /// Current status of the message
    pub status: MessageStatus,
    /// When the message was first queued
    pub queued_at: SystemTime,
    /// Number of processing attempts
    pub attempts: u32,
    /// Last error message if any
    pub last_error: Option<String>,
}

/// Configuration for the relayer service
#[derive(Debug, Clone)]
pub struct RelayerConfig {
    /// Maximum number of concurrent messages to process
    pub max_concurrent_messages: usize,
    /// Maximum number of retry attempts per message
    pub max_retry_attempts: u32,
    /// Base delay between retries in seconds
    pub retry_delay_secs: u64,
    /// Whether to enable automatic pruning of processed messages
    pub enable_auto_pruning: bool,
    /// How long to keep processed messages in history (in hours)
    pub message_history_hours: u64,
} 