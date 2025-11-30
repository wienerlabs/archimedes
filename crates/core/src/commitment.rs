use ark_ed_on_bls12_381::{EdwardsProjective as G, Fr as ScalarField};
use ark_ff::UniformRand;
use ark_serialize::{CanonicalDeserialize, CanonicalSerialize};
use ark_std::rand::Rng;
use ark_std::Zero;

use crate::errors::ArchimedesError;

pub type CommitmentResult<T> = std::result::Result<T, ArchimedesError>;

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct CommitmentParams {
    pub g: G,
    pub h: G,
}

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct Commitment(pub G);

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct Randomness(pub ScalarField);

#[derive(Clone, Debug, PartialEq, Eq, CanonicalSerialize, CanonicalDeserialize)]
pub struct Opening {
    pub value: ScalarField,
    pub randomness: Randomness,
}

impl CommitmentParams {
    pub fn setup<R: Rng>(rng: &mut R) -> CommitmentResult<Self> {
        let g = G::rand(rng);
        let h = G::rand(rng);

        if g == G::zero() || h == G::zero() {
            return Err(ArchimedesError::SetupError(
                "Generator points cannot be identity".to_string(),
            ));
        }

        Ok(Self { g, h })
    }

    pub fn commit<R: Rng>(&self, value: &ScalarField, rng: &mut R) -> CommitmentResult<(Commitment, Randomness)> {
        let r = ScalarField::rand(rng);
        let commitment = self.commit_with_randomness(value, &Randomness(r.clone()))?;
        Ok((commitment, Randomness(r)))
    }

    pub fn commit_with_randomness(&self, value: &ScalarField, randomness: &Randomness) -> CommitmentResult<Commitment> {
        let c = self.g * value + self.h * randomness.0;
        Ok(Commitment(c))
    }

    pub fn verify(&self, commitment: &Commitment, opening: &Opening) -> CommitmentResult<bool> {
        let expected = self.commit_with_randomness(&opening.value, &opening.randomness)?;
        Ok(commitment.0 == expected.0)
    }
}

impl Commitment {
    pub fn zero() -> Self {
        Commitment(G::zero())
    }

    pub fn add(&self, other: &Commitment) -> Commitment {
        Commitment(self.0 + other.0)
    }
}

impl std::ops::Add for Commitment {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Commitment(self.0 + other.0)
    }
}

impl std::ops::Add<&Commitment> for &Commitment {
    type Output = Commitment;
    fn add(self, other: &Commitment) -> Commitment {
        Commitment(self.0 + other.0)
    }
}

impl Randomness {
    pub fn zero() -> Self {
        Randomness(ScalarField::from(0u64))
    }

    pub fn add(&self, other: &Randomness) -> Randomness {
        Randomness(self.0 + other.0)
    }
}

impl std::ops::Add for Randomness {
    type Output = Self;
    fn add(self, other: Self) -> Self {
        Randomness(self.0 + other.0)
    }
}

impl std::ops::Add<&Randomness> for &Randomness {
    type Output = Randomness;
    fn add(self, other: &Randomness) -> Randomness {
        Randomness(self.0 + other.0)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use ark_std::test_rng;

    #[test]
    fn test_commitment_setup() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        assert_ne!(params.g, G::zero());
        assert_ne!(params.h, G::zero());
    }

    #[test]
    fn test_commit_and_verify() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let value = ScalarField::from(42u64);
        let (commitment, randomness) = params.commit(&value, &mut rng).unwrap();
        let opening = Opening { value, randomness };
        assert!(params.verify(&commitment, &opening).unwrap());
    }

    #[test]
    fn test_commitment_binding() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let v1 = ScalarField::from(100u64);
        let v2 = ScalarField::from(200u64);
        let (c1, r1) = params.commit(&v1, &mut rng).unwrap();
        let (c2, _) = params.commit(&v2, &mut rng).unwrap();
        assert_ne!(c1.0, c2.0);
        let wrong_opening = Opening { value: v2, randomness: r1 };
        assert!(!params.verify(&c1, &wrong_opening).unwrap());
    }

    #[test]
    fn test_commitment_homomorphism() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let v1 = ScalarField::from(10u64);
        let v2 = ScalarField::from(20u64);
        let (c1, r1) = params.commit(&v1, &mut rng).unwrap();
        let (c2, r2) = params.commit(&v2, &mut rng).unwrap();
        let c_sum = &c1 + &c2;
        let r_sum = &r1 + &r2;
        let v_sum = v1 + v2;
        let opening = Opening { value: v_sum, randomness: r_sum };
        assert!(params.verify(&c_sum, &opening).unwrap());
    }
}

