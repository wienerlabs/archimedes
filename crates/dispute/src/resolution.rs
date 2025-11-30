use archimedes_core::{ArchimedesError, Commitment, CommitmentParams, Opening};
use archimedes_state::{AccountState, StateTransition};
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, ArchimedesError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeOutcome {
    ProposerCorrect,
    ProposerFaulty,
    InvalidProof,
    Timeout,
}

#[derive(Clone, Debug)]
pub struct SingleStepProof {
    pub index: usize,
    pub pre_state: AccountState,
    pub post_state: AccountState,
    pub commitment: Commitment,
    pub opening: Opening,
}

pub struct DisputeResolver {
    params: CommitmentParams,
}

impl DisputeResolver {
    pub fn new(params: CommitmentParams) -> Self {
        Self { params }
    }

    pub fn verify_single_step(&self, proof: &SingleStepProof) -> Result<DisputeOutcome> {
        if !self.params.verify(&proof.commitment, &proof.opening)? {
            return Ok(DisputeOutcome::InvalidProof);
        }

        let transition = StateTransition::new(
            proof.pre_state.clone(),
            proof.post_state.clone(),
            [0u8; 32],
        );
        let expected_value = transition.to_commitment_value();

        if proof.opening.value != expected_value {
            return Ok(DisputeOutcome::ProposerFaulty);
        }

        Ok(DisputeOutcome::ProposerCorrect)
    }

    pub fn execute_transition(&self, pre: &AccountState, tx_value: u128) -> Result<AccountState> {
        if pre.balance < tx_value {
            return Err(ArchimedesError::DisputeError("Insufficient balance".to_string()));
        }
        Ok(AccountState {
            balance: pre.balance - tx_value,
            nonce: pre.nonce + 1,
            code_hash: pre.code_hash,
            storage_root: pre.storage_root,
        })
    }

    pub fn verify_transition(
        &self,
        pre: &AccountState,
        post: &AccountState,
        tx_value: u128,
    ) -> Result<bool> {
        let expected = self.execute_transition(pre, tx_value)?;
        Ok(expected == *post)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::test_rng;

    #[test]
    fn test_execute_transition() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let resolver = DisputeResolver::new(params);

        let pre = AccountState::new(1000, 0);
        let post = resolver.execute_transition(&pre, 100).unwrap();

        assert_eq!(post.balance, 900);
        assert_eq!(post.nonce, 1);
    }

    #[test]
    fn test_verify_transition() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let resolver = DisputeResolver::new(params);

        let pre = AccountState::new(1000, 0);
        let post = AccountState::new(900, 1);

        assert!(resolver.verify_transition(&pre, &post, 100).unwrap());
        assert!(!resolver.verify_transition(&pre, &post, 50).unwrap());
    }

    #[test]
    fn test_insufficient_balance() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let resolver = DisputeResolver::new(params);

        let pre = AccountState::new(100, 0);
        let result = resolver.execute_transition(&pre, 200);

        assert!(result.is_err());
    }

    #[test]
    fn test_single_step_verification() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let resolver = DisputeResolver::new(params.clone());

        let pre = AccountState::new(1000, 0);
        let post = AccountState::new(900, 1);
        let transition = StateTransition::new(pre.clone(), post.clone(), [0u8; 32]);
        let value = transition.to_commitment_value();

        let (commitment, randomness) = params.commit(&value, &mut rng).unwrap();
        let opening = Opening { value, randomness };

        let proof = SingleStepProof {
            index: 0,
            pre_state: pre,
            post_state: post,
            commitment,
            opening,
        };

        let outcome = resolver.verify_single_step(&proof).unwrap();
        assert_eq!(outcome, DisputeOutcome::ProposerCorrect);
    }
}

