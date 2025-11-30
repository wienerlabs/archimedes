use archimedes_core::{AggregateCommitment, ArchimedesError};
use archimedes_state::CommitmentMerkleTree;
use serde::{Deserialize, Serialize};

type Result<T> = std::result::Result<T, ArchimedesError>;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum BisectionState {
    Initial,
    Challenged,
    BisectLeft,
    BisectRight,
    Resolve,
    Complete(DisputeResult),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum DisputeResult {
    ProposerWins,
    ChallengerWins,
    Timeout,
}

#[derive(Clone, Debug)]
pub struct Challenge {
    pub challenger_id: [u8; 32],
    pub disputed_range: (usize, usize),
    pub claimed_aggregate: AggregateCommitment,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct Response {
    pub proposer_id: [u8; 32],
    pub mid_index: usize,
    pub left_aggregate: AggregateCommitment,
    pub right_aggregate: AggregateCommitment,
    pub timestamp: u64,
}

#[derive(Clone, Debug)]
pub struct BisectionProtocol {
    pub state: BisectionState,
    pub tree: CommitmentMerkleTree,
    pub current_range: (usize, usize),
    pub challenge: Option<Challenge>,
    pub responses: Vec<Response>,
    pub round: usize,
    pub max_rounds: usize,
}

impl BisectionProtocol {
    pub fn new(tree: CommitmentMerkleTree) -> Self {
        let leaf_count = tree.leaf_count();
        let max_rounds = (leaf_count as f64).log2().ceil() as usize + 1;
        Self {
            state: BisectionState::Initial,
            current_range: (0, leaf_count),
            tree,
            challenge: None,
            responses: Vec::new(),
            round: 0,
            max_rounds,
        }
    }

    pub fn initiate_challenge(&mut self, challenge: Challenge) -> Result<()> {
        if self.state != BisectionState::Initial {
            return Err(ArchimedesError::DisputeError("Invalid state for challenge".to_string()));
        }
        let (start, end) = challenge.disputed_range;
        if end > self.tree.leaf_count() || start >= end {
            return Err(ArchimedesError::DisputeError("Invalid dispute range".to_string()));
        }
        self.current_range = (start, end);
        self.challenge = Some(challenge);
        self.state = BisectionState::Challenged;
        Ok(())
    }

    pub fn respond(&mut self, response: Response) -> Result<()> {
        if !matches!(self.state, BisectionState::Challenged | BisectionState::BisectLeft | BisectionState::BisectRight) {
            return Err(ArchimedesError::DisputeError("Invalid state for response".to_string()));
        }
        let (start, end) = self.current_range;
        let mid = response.mid_index;
        if mid <= start || mid >= end {
            return Err(ArchimedesError::DisputeError("Invalid midpoint".to_string()));
        }
        let left_agg = self.tree.range_aggregate(start, mid)?;
        let right_agg = self.tree.range_aggregate(mid, end)?;
        if left_agg.commitment.0 != response.left_aggregate.commitment.0 ||
           right_agg.commitment.0 != response.right_aggregate.commitment.0 {
            self.state = BisectionState::Complete(DisputeResult::ChallengerWins);
            return Ok(());
        }
        self.responses.push(response);
        self.round += 1;
        if end - start <= 2 {
            self.state = BisectionState::Resolve;
        }
        Ok(())
    }

    pub fn select_direction(&mut self, go_left: bool) -> Result<()> {
        if !matches!(self.state, BisectionState::Challenged | BisectionState::BisectLeft | BisectionState::BisectRight) {
            return Err(ArchimedesError::DisputeError("Invalid state".to_string()));
        }
        if self.responses.is_empty() {
            return Err(ArchimedesError::DisputeError("No response to bisect".to_string()));
        }
        let last = self.responses.last().unwrap();
        let (start, end) = self.current_range;
        if go_left {
            self.current_range = (start, last.mid_index);
            self.state = BisectionState::BisectLeft;
        } else {
            self.current_range = (last.mid_index, end);
            self.state = BisectionState::BisectRight;
        }
        if self.current_range.1 - self.current_range.0 <= 1 {
            self.state = BisectionState::Resolve;
        }
        Ok(())
    }

    pub fn is_resolved(&self) -> bool {
        matches!(self.state, BisectionState::Complete(_) | BisectionState::Resolve)
    }

    pub fn disputed_index(&self) -> Option<usize> {
        if self.state == BisectionState::Resolve && self.current_range.1 - self.current_range.0 == 1 {
            Some(self.current_range.0)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_core::{CommitmentChain, CommitmentParams};
    use ark_ed_on_bls12_381::Fr as ScalarField;
    use ark_std::test_rng;

    fn setup_tree(size: usize) -> CommitmentMerkleTree {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let mut chain = CommitmentChain::new(params);
        for i in 1..=size {
            chain.push(ScalarField::from(i as u64), &mut rng).unwrap();
        }
        CommitmentMerkleTree::build(&chain.commitments).unwrap()
    }

    #[test]
    fn test_bisection_init() {
        let tree = setup_tree(8);
        let protocol = BisectionProtocol::new(tree);
        assert_eq!(protocol.state, BisectionState::Initial);
        assert_eq!(protocol.current_range, (0, 8));
    }

    #[test]
    fn test_challenge_initiation() {
        let tree = setup_tree(8);
        let agg = tree.aggregate().clone();
        let mut protocol = BisectionProtocol::new(tree);
        let challenge = Challenge {
            challenger_id: [1u8; 32],
            disputed_range: (0, 8),
            claimed_aggregate: agg,
            timestamp: 0,
        };
        protocol.initiate_challenge(challenge).unwrap();
        assert_eq!(protocol.state, BisectionState::Challenged);
    }
}

