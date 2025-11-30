use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum BondError {
    #[error("Insufficient bond: required {required}, provided {provided}")]
    InsufficientBond { required: u128, provided: u128 },
    #[error("Challenge not found: {0}")]
    ChallengeNotFound(String),
    #[error("Bond already posted for challenge: {0}")]
    BondAlreadyExists(String),
}

type Result<T> = std::result::Result<T, BondError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct ChallengerBond {
    pub challenger_id: String,
    pub challenge_id: String,
    pub amount: u128,
    pub dispute_depth: u32,
    pub forfeited: bool,
}

impl ChallengerBond {
    pub fn new(challenger_id: String, challenge_id: String, amount: u128, dispute_depth: u32) -> Self {
        Self {
            challenger_id,
            challenge_id,
            amount,
            dispute_depth,
            forfeited: false,
        }
    }
}

pub struct BondManager {
    bonds: HashMap<String, ChallengerBond>,
    base_bond: u128,
    depth_multiplier: u128,
}

impl BondManager {
    pub fn new(base_bond: u128, depth_multiplier: u128) -> Self {
        Self {
            bonds: HashMap::new(),
            base_bond,
            depth_multiplier,
        }
    }

    pub fn required_bond(&self, dispute_depth: u32) -> u128 {
        self.base_bond + (dispute_depth as u128 * self.depth_multiplier)
    }

    pub fn post_bond(
        &mut self,
        challenger_id: String,
        challenge_id: String,
        amount: u128,
        dispute_depth: u32,
    ) -> Result<()> {
        if self.bonds.contains_key(&challenge_id) {
            return Err(BondError::BondAlreadyExists(challenge_id));
        }

        let required = self.required_bond(dispute_depth);
        if amount < required {
            return Err(BondError::InsufficientBond { required, provided: amount });
        }

        let bond = ChallengerBond::new(challenger_id, challenge_id.clone(), amount, dispute_depth);
        self.bonds.insert(challenge_id, bond);
        Ok(())
    }

    pub fn forfeit(&mut self, challenge_id: &str) -> Result<u128> {
        let bond = self.bonds.get_mut(challenge_id)
            .ok_or_else(|| BondError::ChallengeNotFound(challenge_id.to_string()))?;
        
        if bond.forfeited {
            return Ok(0);
        }
        
        bond.forfeited = true;
        Ok(bond.amount)
    }

    pub fn return_bond(&mut self, challenge_id: &str) -> Result<u128> {
        let bond = self.bonds.get(challenge_id)
            .ok_or_else(|| BondError::ChallengeNotFound(challenge_id.to_string()))?;
        
        if bond.forfeited {
            return Ok(0);
        }
        
        let amount = bond.amount;
        self.bonds.remove(challenge_id);
        Ok(amount)
    }

    pub fn get_bond(&self, challenge_id: &str) -> Option<&ChallengerBond> {
        self.bonds.get(challenge_id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bond_posting() {
        let mut manager = BondManager::new(100, 10);
        manager.post_bond("challenger1".to_string(), "challenge1".to_string(), 150, 5).unwrap();
        
        let bond = manager.get_bond("challenge1").unwrap();
        assert_eq!(bond.amount, 150);
        assert_eq!(bond.dispute_depth, 5);
    }

    #[test]
    fn test_bond_scaling() {
        let manager = BondManager::new(100, 10);
        assert_eq!(manager.required_bond(0), 100);
        assert_eq!(manager.required_bond(5), 150);
        assert_eq!(manager.required_bond(10), 200);
    }

    #[test]
    fn test_forfeit() {
        let mut manager = BondManager::new(100, 10);
        manager.post_bond("challenger1".to_string(), "challenge1".to_string(), 200, 5).unwrap();
        
        let forfeited = manager.forfeit("challenge1").unwrap();
        assert_eq!(forfeited, 200);
        
        let bond = manager.get_bond("challenge1").unwrap();
        assert!(bond.forfeited);
    }
}

