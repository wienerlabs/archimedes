pub mod aggregation;
pub mod commitment;
pub mod errors;

pub use aggregation::{AggregateCommitment, CommitmentChain};
pub use commitment::{Commitment, CommitmentParams, Opening, Randomness};
pub use errors::{ArchimedesError, Result};

pub mod types {
    pub use ark_ed_on_bls12_381::{EdwardsProjective as G1, Fr as ScalarField};
}

