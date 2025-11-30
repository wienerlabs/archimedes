use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum RewardError {
    #[error("Invalid reward calculation")]
    InvalidCalculation,
    #[error("No funds available for distribution")]
    NoFundsAvailable,
}

type Result<T> = std::result::Result<T, RewardError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeOutcome {
    ChallengerWins,
    ProposerWins,
    Timeout,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct DisputeReward {
    pub challenger_id: String,
    pub proposer_id: String,
    pub outcome: DisputeOutcome,
    pub challenger_reward: u128,
    pub proposer_reward: u128,
    pub protocol_fee: u128,
}

pub struct RewardDistributor {
    protocol_fee_bps: u128, // basis points
    interest_rate_bps: u128,
}

impl RewardDistributor {
    pub fn new(protocol_fee_bps: u128, interest_rate_bps: u128) -> Self {
        Self {
            protocol_fee_bps,
            interest_rate_bps,
        }
    }

    pub fn calculate_reward(
        &self,
        challenger_id: String,
        proposer_id: String,
        outcome: DisputeOutcome,
        stake_amount: u128,
        bond_amount: u128,
        dispute_duration_blocks: u64,
    ) -> Result<DisputeReward> {
        let total_pool = stake_amount + bond_amount;
        let protocol_fee = total_pool * self.protocol_fee_bps / 10000;
        let remaining = total_pool - protocol_fee;

        let interest = stake_amount * self.interest_rate_bps * dispute_duration_blocks as u128 / (10000 * 365 * 24 * 6);

        let (challenger_reward, proposer_reward) = match outcome {
            DisputeOutcome::ChallengerWins => {
                let challenger_gets = remaining.min(stake_amount + interest + bond_amount);
                (challenger_gets, 0)
            }
            DisputeOutcome::ProposerWins => {
                let proposer_gets = remaining.min(stake_amount + bond_amount);
                (0, proposer_gets)
            }
            DisputeOutcome::Timeout => {
                let half = remaining / 2;
                (half, remaining - half)
            }
        };

        Ok(DisputeReward {
            challenger_id,
            proposer_id,
            outcome,
            challenger_reward,
            proposer_reward,
            protocol_fee,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_challenger_wins() {
        let distributor = RewardDistributor::new(100, 500); // 1% fee, 5% annual interest
        
        let reward = distributor.calculate_reward(
            "challenger1".to_string(),
            "proposer1".to_string(),
            DisputeOutcome::ChallengerWins,
            1000,
            100,
            100,
        ).unwrap();

        assert_eq!(reward.outcome, DisputeOutcome::ChallengerWins);
        assert!(reward.challenger_reward > 0);
        assert_eq!(reward.proposer_reward, 0);
        assert!(reward.protocol_fee > 0);
    }

    #[test]
    fn test_proposer_wins() {
        let distributor = RewardDistributor::new(100, 500);
        
        let reward = distributor.calculate_reward(
            "challenger1".to_string(),
            "proposer1".to_string(),
            DisputeOutcome::ProposerWins,
            1000,
            100,
            100,
        ).unwrap();

        assert_eq!(reward.outcome, DisputeOutcome::ProposerWins);
        assert_eq!(reward.challenger_reward, 0);
        assert!(reward.proposer_reward > 0);
    }

    #[test]
    fn test_timeout_split() {
        let distributor = RewardDistributor::new(100, 500);
        
        let reward = distributor.calculate_reward(
            "challenger1".to_string(),
            "proposer1".to_string(),
            DisputeOutcome::Timeout,
            1000,
            100,
            100,
        ).unwrap();

        assert_eq!(reward.outcome, DisputeOutcome::Timeout);
        assert!(reward.challenger_reward > 0);
        assert!(reward.proposer_reward > 0);
    }
}

