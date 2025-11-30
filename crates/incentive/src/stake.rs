use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StakeError {
    #[error("Insufficient stake: required {required}, available {available}")]
    InsufficientStake { required: u128, available: u128 },
    #[error("Proposer not found: {0}")]
    ProposerNotFound(String),
    #[error("Stake already exists for proposer: {0}")]
    StakeAlreadyExists(String),
    #[error("Invalid stake amount")]
    InvalidAmount,
}

type Result<T> = std::result::Result<T, StakeError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct StakeInfo {
    pub proposer_id: String,
    pub amount: u128,
    pub commitment_value: u128,
    pub locked_until: u64,
    pub slashed: bool,
}

impl StakeInfo {
    pub fn new(proposer_id: String, amount: u128, commitment_value: u128, lock_duration: u64) -> Self {
        Self {
            proposer_id,
            amount,
            commitment_value,
            locked_until: lock_duration,
            slashed: false,
        }
    }

    pub fn is_locked(&self, current_time: u64) -> bool {
        current_time < self.locked_until && !self.slashed
    }
}

pub struct StakeManager {
    stakes: HashMap<String, StakeInfo>,
    min_stake_ratio: u128, // basis points (1/10000)
}

impl StakeManager {
    pub fn new(min_stake_ratio: u128) -> Self {
        Self {
            stakes: HashMap::new(),
            min_stake_ratio,
        }
    }

    pub fn required_stake(&self, commitment_value: u128) -> u128 {
        commitment_value * self.min_stake_ratio / 10000
    }

    pub fn deposit(&mut self, proposer_id: String, amount: u128, commitment_value: u128, lock_duration: u64) -> Result<()> {
        if self.stakes.contains_key(&proposer_id) {
            return Err(StakeError::StakeAlreadyExists(proposer_id));
        }

        let required = self.required_stake(commitment_value);
        if amount < required {
            return Err(StakeError::InsufficientStake { required, available: amount });
        }

        let stake = StakeInfo::new(proposer_id.clone(), amount, commitment_value, lock_duration);
        self.stakes.insert(proposer_id, stake);
        Ok(())
    }

    pub fn slash(&mut self, proposer_id: &str) -> Result<u128> {
        let stake = self.stakes.get_mut(proposer_id)
            .ok_or_else(|| StakeError::ProposerNotFound(proposer_id.to_string()))?;
        
        if stake.slashed {
            return Ok(0);
        }
        
        stake.slashed = true;
        Ok(stake.amount)
    }

    pub fn withdraw(&mut self, proposer_id: &str, current_time: u64) -> Result<u128> {
        let stake = self.stakes.get(proposer_id)
            .ok_or_else(|| StakeError::ProposerNotFound(proposer_id.to_string()))?;
        
        if stake.is_locked(current_time) {
            return Err(StakeError::InvalidAmount);
        }
        
        if stake.slashed {
            return Ok(0);
        }
        
        let amount = stake.amount;
        self.stakes.remove(proposer_id);
        Ok(amount)
    }

    pub fn get_stake(&self, proposer_id: &str) -> Option<&StakeInfo> {
        self.stakes.get(proposer_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stake_deposit() {
        let mut manager = StakeManager::new(100); // 1%
        manager.deposit("proposer1".to_string(), 1000, 10000, 100).unwrap();
        
        let stake = manager.get_stake("proposer1").unwrap();
        assert_eq!(stake.amount, 1000);
    }

    #[test]
    fn test_insufficient_stake() {
        let mut manager = StakeManager::new(100);
        let result = manager.deposit("proposer1".to_string(), 50, 10000, 100);
        assert!(matches!(result, Err(StakeError::InsufficientStake { .. })));
    }

    #[test]
    fn test_slash() {
        let mut manager = StakeManager::new(100);
        manager.deposit("proposer1".to_string(), 1000, 10000, 100).unwrap();
        
        let slashed = manager.slash("proposer1").unwrap();
        assert_eq!(slashed, 1000);
        
        let stake = manager.get_stake("proposer1").unwrap();
        assert!(stake.slashed);
    }
}

