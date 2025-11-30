use crate::erasure::EncodedShard;
use crate::storage::ContentId;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SamplingError {
    #[error("Sample verification failed")]
    VerificationFailed,
    #[error("Insufficient samples: have {have}, need {need}")]
    InsufficientSamples { have: usize, need: usize },
    #[error("Invalid merkle proof")]
    InvalidMerkleProof,
}

type Result<T> = std::result::Result<T, SamplingError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SampleProof {
    pub shard_index: usize,
    pub shard_hash: [u8; 32],
    pub merkle_path: Vec<[u8; 32]>,
}

pub struct AvailabilitySampler {
    required_samples: usize,
    total_shards: usize,
}

impl AvailabilitySampler {
    pub fn new(required_samples: usize, total_shards: usize) -> Self {
        Self { required_samples, total_shards }
    }

    pub fn generate_sample_indices(&self, seed: &[u8]) -> Vec<usize> {
        let mut indices = Vec::with_capacity(self.required_samples);
        let mut hasher = Sha256::new();
        hasher.update(seed);
        let mut current = hasher.finalize();

        while indices.len() < self.required_samples {
            let idx = u32::from_be_bytes([current[0], current[1], current[2], current[3]]) as usize;
            let shard_idx = idx % self.total_shards;
            
            if !indices.contains(&shard_idx) {
                indices.push(shard_idx);
            }
            
            let mut next_hasher = Sha256::new();
            next_hasher.update(&current);
            current = next_hasher.finalize();
        }

        indices
    }

    pub fn create_proof(shard: &EncodedShard, all_shards: &[EncodedShard]) -> SampleProof {
        let mut hasher = Sha256::new();
        hasher.update(&shard.data);
        let hash = hasher.finalize();
        let mut shard_hash = [0u8; 32];
        shard_hash.copy_from_slice(&hash);

        let merkle_path = Self::build_merkle_path(shard.index, all_shards);

        SampleProof {
            shard_index: shard.index,
            shard_hash,
            merkle_path,
        }
    }

    fn build_merkle_path(index: usize, shards: &[EncodedShard]) -> Vec<[u8; 32]> {
        let hashes: Vec<[u8; 32]> = shards.iter().map(|s| {
            let mut hasher = Sha256::new();
            hasher.update(&s.data);
            let result = hasher.finalize();
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&result);
            hash
        }).collect();

        let mut path = Vec::new();
        let mut level = hashes;
        let mut idx = index;

        while level.len() > 1 {
            let sibling_idx = if idx % 2 == 0 { idx + 1 } else { idx - 1 };
            if sibling_idx < level.len() {
                path.push(level[sibling_idx]);
            }
            
            level = level.chunks(2).map(|pair| {
                let mut hasher = Sha256::new();
                hasher.update(&pair[0]);
                if pair.len() > 1 {
                    hasher.update(&pair[1]);
                }
                let result = hasher.finalize();
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&result);
                hash
            }).collect();
            
            idx /= 2;
        }

        path
    }

    pub fn verify_proof(&self, proof: &SampleProof, root: &ContentId) -> Result<bool> {
        let mut current = proof.shard_hash;
        let mut idx = proof.shard_index;

        for sibling in &proof.merkle_path {
            let mut hasher = Sha256::new();
            if idx % 2 == 0 {
                hasher.update(&current);
                hasher.update(sibling);
            } else {
                hasher.update(sibling);
                hasher.update(&current);
            }
            let result = hasher.finalize();
            current.copy_from_slice(&result);
            idx /= 2;
        }

        Ok(current == root.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::erasure::ErasureEncoder;

    #[test]
    fn test_sample_generation() {
        let sampler = AvailabilitySampler::new(5, 16);
        let indices = sampler.generate_sample_indices(b"test_seed");
        
        assert_eq!(indices.len(), 5);
        for idx in &indices {
            assert!(*idx < 16);
        }
    }

    #[test]
    fn test_create_proof() {
        let encoder = ErasureEncoder::new(4, 2);
        let shards = encoder.encode(b"test data").unwrap();
        
        let proof = AvailabilitySampler::create_proof(&shards[0], &shards);
        assert_eq!(proof.shard_index, 0);
        assert!(!proof.merkle_path.is_empty());
    }
}

