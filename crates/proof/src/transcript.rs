use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum TranscriptError {
    #[error("Invalid transcript entry")]
    InvalidEntry,
    #[error("Transcript verification failed")]
    VerificationFailed,
}

type Result<T> = std::result::Result<T, TranscriptError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TranscriptEntry {
    pub index: u64,
    pub label: String,
    pub data_hash: [u8; 32],
    pub running_hash: [u8; 32],
}

#[derive(Clone, Debug)]
pub struct ProofTranscript {
    entries: Vec<TranscriptEntry>,
    current_hash: [u8; 32],
}

impl ProofTranscript {
    pub fn new() -> Self {
        Self {
            entries: Vec::new(),
            current_hash: [0u8; 32],
        }
    }

    pub fn append(&mut self, label: &str, data: &[u8]) {
        let mut data_hasher = Sha256::new();
        data_hasher.update(data);
        let data_result = data_hasher.finalize();
        let mut data_hash = [0u8; 32];
        data_hash.copy_from_slice(&data_result);

        let mut running_hasher = Sha256::new();
        running_hasher.update(&self.current_hash);
        running_hasher.update(&data_hash);
        let running_result = running_hasher.finalize();
        let mut running_hash = [0u8; 32];
        running_hash.copy_from_slice(&running_result);

        let entry = TranscriptEntry {
            index: self.entries.len() as u64,
            label: label.to_string(),
            data_hash,
            running_hash,
        };

        self.current_hash = running_hash;
        self.entries.push(entry);
    }

    pub fn challenge(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(b"challenge");
        hasher.update(&self.current_hash);
        let result = hasher.finalize();
        let mut challenge = [0u8; 32];
        challenge.copy_from_slice(&result);
        challenge
    }

    pub fn verify(&self) -> Result<bool> {
        let mut expected_hash = [0u8; 32];

        for entry in &self.entries {
            let mut hasher = Sha256::new();
            hasher.update(&expected_hash);
            hasher.update(&entry.data_hash);
            let result = hasher.finalize();
            expected_hash.copy_from_slice(&result);

            if expected_hash != entry.running_hash {
                return Err(TranscriptError::VerificationFailed);
            }
        }

        Ok(true)
    }

    pub fn entries(&self) -> &[TranscriptEntry] {
        &self.entries
    }

    pub fn current_hash(&self) -> [u8; 32] {
        self.current_hash
    }
}

impl Default for ProofTranscript {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transcript_append() {
        let mut transcript = ProofTranscript::new();
        transcript.append("step1", b"data1");
        transcript.append("step2", b"data2");

        assert_eq!(transcript.entries().len(), 2);
        assert_ne!(transcript.current_hash(), [0u8; 32]);
    }

    #[test]
    fn test_transcript_verify() {
        let mut transcript = ProofTranscript::new();
        transcript.append("init", b"genesis");
        transcript.append("transition", b"state_change");

        let result = transcript.verify();
        assert!(result.is_ok());
    }

    #[test]
    fn test_challenge_determinism() {
        let mut t1 = ProofTranscript::new();
        t1.append("data", b"same_data");

        let mut t2 = ProofTranscript::new();
        t2.append("data", b"same_data");

        assert_eq!(t1.challenge(), t2.challenge());
    }

    #[test]
    fn test_challenge_uniqueness() {
        let mut t1 = ProofTranscript::new();
        t1.append("data", b"data1");

        let mut t2 = ProofTranscript::new();
        t2.append("data", b"data2");

        assert_ne!(t1.challenge(), t2.challenge());
    }
}

