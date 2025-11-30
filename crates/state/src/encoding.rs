use ark_ed_on_bls12_381::Fr as ScalarField;
use ark_ff::PrimeField;
use archimedes_core::ArchimedesError;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, ArchimedesError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct AccountState {
    pub balance: u128,
    pub nonce: u64,
    pub code_hash: [u8; 32],
    pub storage_root: [u8; 32],
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StateTransition {
    pub pre_state: AccountState,
    pub post_state: AccountState,
    pub tx_hash: [u8; 32],
}

impl AccountState {
    pub fn new(balance: u128, nonce: u64) -> Self {
        Self {
            balance,
            nonce,
            code_hash: [0u8; 32],
            storage_root: [0u8; 32],
        }
    }

    pub fn to_field_elements(&self) -> Vec<ScalarField> {
        let mut elements = Vec::with_capacity(4);
        elements.push(ScalarField::from(self.balance as u64));
        elements.push(ScalarField::from(self.nonce));
        elements.push(bytes_to_field(&self.code_hash));
        elements.push(bytes_to_field(&self.storage_root));
        elements
    }

    pub fn hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.balance.to_be_bytes());
        hasher.update(self.nonce.to_be_bytes());
        hasher.update(self.code_hash);
        hasher.update(self.storage_root);
        hasher.finalize().into()
    }

    pub fn to_commitment_value(&self) -> ScalarField {
        bytes_to_field(&self.hash())
    }
}

impl StateTransition {
    pub fn new(pre_state: AccountState, post_state: AccountState, tx_hash: [u8; 32]) -> Self {
        Self { pre_state, post_state, tx_hash }
    }

    pub fn transition_hash(&self) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(self.pre_state.hash());
        hasher.update(self.post_state.hash());
        hasher.update(self.tx_hash);
        hasher.finalize().into()
    }

    pub fn to_commitment_value(&self) -> ScalarField {
        bytes_to_field(&self.transition_hash())
    }
}

pub fn bytes_to_field(bytes: &[u8; 32]) -> ScalarField {
    let mut truncated = [0u8; 31];
    truncated.copy_from_slice(&bytes[..31]);
    ScalarField::from_le_bytes_mod_order(&truncated)
}

pub fn encode_state_batch(states: &[AccountState]) -> Result<Vec<ScalarField>> {
    if states.is_empty() {
        return Err(ArchimedesError::StateEncodingError("Empty state batch".to_string()));
    }
    Ok(states.iter().map(|s| s.to_commitment_value()).collect())
}

pub fn encode_transitions(transitions: &[StateTransition]) -> Result<Vec<ScalarField>> {
    if transitions.is_empty() {
        return Err(ArchimedesError::StateEncodingError("Empty transitions".to_string()));
    }
    Ok(transitions.iter().map(|t| t.to_commitment_value()).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_state_encoding() {
        let state = AccountState::new(1000, 5);
        let elements = state.to_field_elements();
        assert_eq!(elements.len(), 4);
        assert_eq!(elements[0], ScalarField::from(1000u64));
        assert_eq!(elements[1], ScalarField::from(5u64));
    }

    #[test]
    fn test_account_hash_determinism() {
        let s1 = AccountState::new(100, 1);
        let s2 = AccountState::new(100, 1);
        assert_eq!(s1.hash(), s2.hash());
        let s3 = AccountState::new(101, 1);
        assert_ne!(s1.hash(), s3.hash());
    }

    #[test]
    fn test_state_transition() {
        let pre = AccountState::new(1000, 0);
        let post = AccountState::new(900, 1);
        let tx = StateTransition::new(pre, post, [1u8; 32]);
        let h1 = tx.transition_hash();
        let h2 = tx.transition_hash();
        assert_eq!(h1, h2);
    }

    #[test]
    fn test_encode_batch() {
        let states = vec![AccountState::new(100, 0), AccountState::new(200, 1)];
        let encoded = encode_state_batch(&states).unwrap();
        assert_eq!(encoded.len(), 2);
    }
}

