use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum RelayerError {
    #[error("Message not found: {0}")]
    MessageNotFound(Uuid),

    #[error("Chain adapter error: {0}")]
    ChainAdapter(#[from] frostgate_icap::AdapterError),

    #[error("Prover error: {0}")]
    Prover(String),

    #[error("Configuration error: {0}")]
    Configuration(String),

    #[error("Queue error: {0}")]
    Queue(String),

    #[error("Message validation failed: {0}")]
    ValidationError(String),

    #[error("Timeout waiting for {operation} after {seconds} seconds")]
    Timeout {
        operation: String,
        seconds: u64,
    },

    #[error(transparent)]
    Other(#[from] anyhow::Error),
} 