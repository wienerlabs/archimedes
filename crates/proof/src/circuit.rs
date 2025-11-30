use crate::witness::{TransitionOperation, TransitionWitness};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum CircuitError {
    #[error("Constraint violation: {0}")]
    ConstraintViolation(String),
    #[error("Invalid input")]
    InvalidInput,
    #[error("Circuit compilation failed")]
    CompilationFailed,
}

type Result<T> = std::result::Result<T, CircuitError>;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CircuitInput {
    pub pre_state_hash: [u8; 32],
    pub post_state_hash: [u8; 32],
    pub operation_hash: [u8; 32],
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Constraint {
    pub left: ConstraintTerm,
    pub right: ConstraintTerm,
    pub output: ConstraintTerm,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum ConstraintTerm {
    Constant(u64),
    Variable(usize),
    LinearCombination(Vec<(u64, usize)>),
}

#[derive(Clone, Debug)]
pub struct TransitionCircuit {
    pub constraints: Vec<Constraint>,
    pub input: CircuitInput,
    pub num_variables: usize,
}

impl TransitionCircuit {
    pub fn from_witness(witness: &TransitionWitness) -> Result<Self> {
        let pre_hash = witness.pre_state.hash();
        let post_hash = witness.post_state.hash();
        
        let operation_hash = Self::hash_operation(&witness.operation);

        let input = CircuitInput {
            pre_state_hash: pre_hash,
            post_state_hash: post_hash,
            operation_hash,
        };

        let mut constraints = Vec::new();

        constraints.push(Constraint {
            left: ConstraintTerm::Variable(0),
            right: ConstraintTerm::Constant(1),
            output: ConstraintTerm::Variable(1),
        });

        constraints.push(Constraint {
            left: ConstraintTerm::Variable(1),
            right: ConstraintTerm::Variable(2),
            output: ConstraintTerm::Variable(3),
        });

        constraints.push(Constraint {
            left: ConstraintTerm::LinearCombination(vec![(1, 0), (1, 1)]),
            right: ConstraintTerm::Constant(1),
            output: ConstraintTerm::Variable(4),
        });

        Ok(Self {
            constraints,
            input,
            num_variables: 5,
        })
    }

    fn hash_operation(op: &TransitionOperation) -> [u8; 32] {
        let mut hasher = Sha256::new();
        match op {
            TransitionOperation::Transfer { amount } => {
                hasher.update(b"transfer");
                hasher.update(&amount.to_le_bytes());
            }
            TransitionOperation::NonceIncrement => {
                hasher.update(b"nonce_inc");
            }
            TransitionOperation::StorageWrite { key, value } => {
                hasher.update(b"storage_write");
                hasher.update(key);
                hasher.update(value);
            }
        }
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    pub fn verify_constraints(&self, assignment: &[u64]) -> Result<bool> {
        if assignment.len() < self.num_variables {
            return Err(CircuitError::InvalidInput);
        }

        for (i, constraint) in self.constraints.iter().enumerate() {
            let left = self.evaluate_term(&constraint.left, assignment);
            let right = self.evaluate_term(&constraint.right, assignment);
            let output = self.evaluate_term(&constraint.output, assignment);

            if left * right != output {
                return Err(CircuitError::ConstraintViolation(
                    format!("Constraint {} failed: {} * {} != {}", i, left, right, output)
                ));
            }
        }

        Ok(true)
    }

    fn evaluate_term(&self, term: &ConstraintTerm, assignment: &[u64]) -> u64 {
        match term {
            ConstraintTerm::Constant(c) => *c,
            ConstraintTerm::Variable(idx) => assignment[*idx],
            ConstraintTerm::LinearCombination(terms) => {
                terms.iter().map(|(coeff, idx)| coeff * assignment[*idx]).sum()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_state::AccountState;
    use crate::witness::WitnessGenerator;

    fn test_account(balance: u128) -> AccountState {
        AccountState {
            balance,
            nonce: 0,
            code_hash: [0u8; 32],
            storage_root: [0u8; 32],
        }
    }

    #[test]
    fn test_circuit_from_witness() {
        let from = test_account(1000);
        let to = test_account(500);

        let witness = WitnessGenerator::generate_transfer(from, to, 100).unwrap();
        let circuit = TransitionCircuit::from_witness(&witness).unwrap();

        assert!(!circuit.constraints.is_empty());
        assert_eq!(circuit.num_variables, 5);
    }

    #[test]
    fn test_constraint_verification() {
        let from = test_account(1000);
        let to = test_account(500);

        let witness = WitnessGenerator::generate_transfer(from, to, 100).unwrap();
        let circuit = TransitionCircuit::from_witness(&witness).unwrap();

        let assignment = vec![2, 2, 2, 4, 4];
        let result = circuit.verify_constraints(&assignment);
        assert!(result.is_ok());
    }
}

