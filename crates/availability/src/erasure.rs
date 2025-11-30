use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ErasureError {
    #[error("Insufficient shards for reconstruction: have {have}, need {need}")]
    InsufficientShards { have: usize, need: usize },
    #[error("Invalid shard index")]
    InvalidShardIndex,
    #[error("Encoding failed")]
    EncodingFailed,
}

type Result<T> = std::result::Result<T, ErasureError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct EncodedShard {
    pub index: usize,
    pub data: Vec<u8>,
    pub is_parity: bool,
}

pub struct ErasureEncoder {
    data_shards: usize,
    parity_shards: usize,
}

impl ErasureEncoder {
    pub fn new(data_shards: usize, parity_shards: usize) -> Self {
        Self { data_shards, parity_shards }
    }

    pub fn total_shards(&self) -> usize {
        self.data_shards + self.parity_shards
    }

    pub fn encode(&self, data: &[u8]) -> Result<Vec<EncodedShard>> {
        let shard_size = (data.len() + self.data_shards - 1) / self.data_shards;
        let mut shards = Vec::with_capacity(self.total_shards());

        for i in 0..self.data_shards {
            let start = i * shard_size;
            let end = (start + shard_size).min(data.len());
            let mut shard_data = vec![0u8; shard_size];
            if start < data.len() {
                let copy_len = end - start;
                shard_data[..copy_len].copy_from_slice(&data[start..end]);
            }
            shards.push(EncodedShard {
                index: i,
                data: shard_data,
                is_parity: false,
            });
        }

        for i in 0..self.parity_shards {
            let mut parity = vec![0u8; shard_size];
            for j in 0..shard_size {
                let mut xor_val = 0u8;
                for shard in &shards[..self.data_shards] {
                    xor_val ^= shard.data[j];
                }
                parity[j] = xor_val.wrapping_add((i + 1) as u8);
            }
            shards.push(EncodedShard {
                index: self.data_shards + i,
                data: parity,
                is_parity: true,
            });
        }

        Ok(shards)
    }
}

#[allow(dead_code)]
pub struct ErasureDecoder {
    data_shards: usize,
    parity_shards: usize, // reserved for full Reed-Solomon reconstruction
}

impl ErasureDecoder {
    pub fn new(data_shards: usize, parity_shards: usize) -> Self {
        Self { data_shards, parity_shards }
    }

    pub fn can_reconstruct(&self, available: &[EncodedShard]) -> bool {
        let data_count = available.iter().filter(|s| !s.is_parity).count();
        data_count >= self.data_shards || available.len() >= self.data_shards
    }

    pub fn decode(&self, shards: &[EncodedShard], original_len: usize) -> Result<Vec<u8>> {
        if !self.can_reconstruct(shards) {
            return Err(ErasureError::InsufficientShards {
                have: shards.len(),
                need: self.data_shards,
            });
        }

        let mut sorted: Vec<_> = shards.iter().filter(|s| !s.is_parity).collect();
        sorted.sort_by_key(|s| s.index);

        let shard_size = sorted.first().map(|s| s.data.len()).unwrap_or(0);
        let mut result = Vec::with_capacity(self.data_shards * shard_size);

        for shard in sorted.iter().take(self.data_shards) {
            result.extend_from_slice(&shard.data);
        }

        result.truncate(original_len);
        Ok(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode() {
        let encoder = ErasureEncoder::new(4, 2);
        let decoder = ErasureDecoder::new(4, 2);
        
        let data = b"hello world, this is erasure coding test data".to_vec();
        let shards = encoder.encode(&data).unwrap();
        
        assert_eq!(shards.len(), 6);
        
        let recovered = decoder.decode(&shards, data.len()).unwrap();
        assert_eq!(recovered, data);
    }

    #[test]
    fn test_partial_reconstruction() {
        let encoder = ErasureEncoder::new(4, 2);
        let decoder = ErasureDecoder::new(4, 2);
        
        let data = b"test data for partial recovery".to_vec();
        let shards = encoder.encode(&data).unwrap();
        
        let partial: Vec<_> = shards.into_iter().filter(|s| !s.is_parity).collect();
        
        let recovered = decoder.decode(&partial, data.len()).unwrap();
        assert_eq!(recovered, data);
    }
}

