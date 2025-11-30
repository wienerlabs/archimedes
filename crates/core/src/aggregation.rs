use ark_ed_on_bls12_381::Fr as ScalarField;

use crate::commitment::{Commitment, CommitmentParams, Opening, Randomness};
use crate::errors::ArchimedesError;

type Result<T> = std::result::Result<T, ArchimedesError>;

#[derive(Clone, Debug)]
pub struct AggregateCommitment {
    pub commitment: Commitment,
    pub count: usize,
}

#[derive(Clone, Debug)]
pub struct CommitmentChain {
    pub params: CommitmentParams,
    pub commitments: Vec<Commitment>,
    pub randomness: Vec<Randomness>,
    pub values: Vec<ScalarField>,
}

impl AggregateCommitment {
    pub fn empty() -> Self {
        Self {
            commitment: Commitment::zero(),
            count: 0,
        }
    }

    pub fn from_commitments(commitments: &[Commitment]) -> Self {
        let mut agg = Commitment::zero();
        for c in commitments {
            agg = agg.add(c);
        }
        Self {
            commitment: agg,
            count: commitments.len(),
        }
    }

    pub fn add(&self, other: &Commitment) -> Self {
        Self {
            commitment: self.commitment.add(other),
            count: self.count + 1,
        }
    }

    pub fn merge(&self, other: &AggregateCommitment) -> Self {
        Self {
            commitment: self.commitment.add(&other.commitment),
            count: self.count + other.count,
        }
    }
}

impl CommitmentChain {
    pub fn new(params: CommitmentParams) -> Self {
        Self {
            params,
            commitments: Vec::new(),
            randomness: Vec::new(),
            values: Vec::new(),
        }
    }

    pub fn push<R: ark_std::rand::Rng>(&mut self, value: ScalarField, rng: &mut R) -> Result<&Commitment> {
        let (commitment, randomness) = self.params.commit(&value, rng)?;
        self.commitments.push(commitment);
        self.randomness.push(randomness);
        self.values.push(value);
        Ok(self.commitments.last().unwrap())
    }

    pub fn aggregate(&self) -> AggregateCommitment {
        AggregateCommitment::from_commitments(&self.commitments)
    }

    pub fn aggregate_range(&self, start: usize, end: usize) -> Result<AggregateCommitment> {
        if end > self.commitments.len() || start > end {
            return Err(ArchimedesError::AggregationError("Invalid range".to_string()));
        }
        Ok(AggregateCommitment::from_commitments(&self.commitments[start..end]))
    }

    pub fn aggregate_randomness(&self) -> Randomness {
        let mut r_agg = Randomness::zero();
        for r in &self.randomness {
            r_agg = r_agg.add(r);
        }
        r_agg
    }

    pub fn aggregate_value(&self) -> ScalarField {
        self.values.iter().fold(ScalarField::from(0u64), |acc, v| acc + v)
    }

    pub fn verify_aggregate(&self, aggregate: &AggregateCommitment) -> Result<bool> {
        let v_sum = self.aggregate_value();
        let r_sum = self.aggregate_randomness();
        let opening = Opening {
            value: v_sum,
            randomness: r_sum,
        };
        self.params.verify(&aggregate.commitment, &opening)
    }

    pub fn len(&self) -> usize {
        self.commitments.len()
    }

    pub fn is_empty(&self) -> bool {
        self.commitments.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::test_rng;

    #[test]
    fn test_aggregate_empty() {
        let agg = AggregateCommitment::empty();
        assert_eq!(agg.count, 0);
    }

    #[test]
    fn test_commitment_chain() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let mut chain = CommitmentChain::new(params);
        for i in 1..=5 {
            chain.push(ScalarField::from(i as u64), &mut rng).unwrap();
        }
        assert_eq!(chain.len(), 5);
        let agg = chain.aggregate();
        assert_eq!(agg.count, 5);
        assert!(chain.verify_aggregate(&agg).unwrap());
    }

    #[test]
    fn test_aggregate_range() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let mut chain = CommitmentChain::new(params);
        for i in 1..=10 {
            chain.push(ScalarField::from(i as u64), &mut rng).unwrap();
        }
        let partial = chain.aggregate_range(0, 5).unwrap();
        assert_eq!(partial.count, 5);
    }

    #[test]
    fn test_aggregate_homomorphism() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let mut chain = CommitmentChain::new(params);
        let values: Vec<u64> = vec![10, 20, 30, 40, 50];
        for v in &values {
            chain.push(ScalarField::from(*v), &mut rng).unwrap();
        }
        let agg = chain.aggregate();
        let expected_sum: u64 = values.iter().sum();
        assert_eq!(chain.aggregate_value(), ScalarField::from(expected_sum));
        assert!(chain.verify_aggregate(&agg).unwrap());
    }
}

