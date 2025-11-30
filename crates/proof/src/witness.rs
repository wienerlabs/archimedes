use archimedes_state::AccountState;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum WitnessError {
    #[error("Invalid state transition")]
    InvalidTransition,
    #[error("Witness generation failed: {0}")]
    GenerationFailed(String),
    #[error("Missing intermediate value")]
    MissingValue,
}

type Result<T> = std::result::Result<T, WitnessError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct TransitionWitness {
    pub pre_state: AccountState,
    pub post_state: AccountState,
    pub operation: TransitionOperation,
    pub intermediate_values: Vec<IntermediateValue>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum TransitionOperation {
    Transfer { amount: u128 },
    NonceIncrement,
    StorageWrite { key: [u8; 32], value: [u8; 32] },
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct IntermediateValue {
    pub step: u32,
    pub description: String,
    pub value_hash: [u8; 32],
}

impl TransitionWitness {
    pub fn compute_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(&self.pre_state.hash());
        hasher.update(&self.post_state.hash());
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }
}

pub struct WitnessGenerator;

impl WitnessGenerator {
    pub fn generate_transfer(
        from_state: AccountState,
        to_state: AccountState,
        amount: u128,
    ) -> Result<TransitionWitness> {
        if from_state.balance < amount {
            return Err(WitnessError::InvalidTransition);
        }

        let mut intermediates = Vec::new();
        
        let mut step1_hasher = Sha256::new();
        step1_hasher.update(&from_state.balance.to_le_bytes());
        step1_hasher.update(&amount.to_le_bytes());
        let step1_result = step1_hasher.finalize();
        let mut step1_hash = [0u8; 32];
        step1_hash.copy_from_slice(&step1_result);
        intermediates.push(IntermediateValue {
            step: 1,
            description: "balance_check".to_string(),
            value_hash: step1_hash,
        });

        let new_from_balance = from_state.balance - amount;
        let mut step2_hasher = Sha256::new();
        step2_hasher.update(&new_from_balance.to_le_bytes());
        let step2_result = step2_hasher.finalize();
        let mut step2_hash = [0u8; 32];
        step2_hash.copy_from_slice(&step2_result);
        intermediates.push(IntermediateValue {
            step: 2,
            description: "from_balance_update".to_string(),
            value_hash: step2_hash,
        });

        let new_to_balance = to_state.balance + amount;
        let mut step3_hasher = Sha256::new();
        step3_hasher.update(&new_to_balance.to_le_bytes());
        let step3_result = step3_hasher.finalize();
        let mut step3_hash = [0u8; 32];
        step3_hash.copy_from_slice(&step3_result);
        intermediates.push(IntermediateValue {
            step: 3,
            description: "to_balance_update".to_string(),
            value_hash: step3_hash,
        });

        let post_from = AccountState {
            balance: new_from_balance,
            nonce: from_state.nonce + 1,
            code_hash: from_state.code_hash,
            storage_root: from_state.storage_root,
        };

        Ok(TransitionWitness {
            pre_state: from_state.clone(),
            post_state: post_from,
            operation: TransitionOperation::Transfer {
                amount,
            },
            intermediate_values: intermediates,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_account(balance: u128) -> AccountState {
        AccountState {
            balance,
            nonce: 0,
            code_hash: [0u8; 32],
            storage_root: [0u8; 32],
        }
    }

    #[test]
    fn test_generate_transfer_witness() {
        let from = test_account(1000);
        let to = test_account(500);

        let witness = WitnessGenerator::generate_transfer(from, to, 100).unwrap();

        assert_eq!(witness.intermediate_values.len(), 3);
        assert_eq!(witness.post_state.balance, 900);
        assert_eq!(witness.post_state.nonce, 1);
    }

    #[test]
    fn test_insufficient_balance() {
        let from = test_account(100);
        let to = test_account(500);

        let result = WitnessGenerator::generate_transfer(from, to, 200);
        assert!(matches!(result, Err(WitnessError::InvalidTransition)));
    }
}

