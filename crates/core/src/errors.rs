use thiserror::Error;

#[derive(Error, Debug, Clone)]
pub enum ArchimedesError {
    #[error("Commitment setup failed: {0}")]
    SetupError(String),

    #[error("Commitment generation failed: {0}")]
    CommitmentError(String),

    #[error("Commitment verification failed: {0}")]
    VerificationError(String),

    #[error("Aggregation failed: {0}")]
    AggregationError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("State encoding error: {0}")]
    StateEncodingError(String),

    #[error("Merkle tree error: {0}")]
    MerkleTreeError(String),

    #[error("Dispute resolution error: {0}")]
    DisputeError(String),
}

