use archimedes_core::{AggregateCommitment, ArchimedesError, Commitment};
use sha2::{Digest, Sha256};

type Result<T> = std::result::Result<T, ArchimedesError>;

#[derive(Clone, Debug)]
pub struct MerkleNode {
    pub hash: [u8; 32],
    pub aggregate: AggregateCommitment,
}

#[derive(Clone, Debug)]
pub struct CommitmentMerkleTree {
    nodes: Vec<Vec<MerkleNode>>,
    leaf_count: usize,
}

impl MerkleNode {
    pub fn leaf(commitment: &Commitment, index: usize) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(index.to_be_bytes());
        let mut commitment_bytes = Vec::new();
        ark_serialize::CanonicalSerialize::serialize_compressed(&commitment.0, &mut commitment_bytes).unwrap();
        hasher.update(&commitment_bytes);
        Self {
            hash: hasher.finalize().into(),
            aggregate: AggregateCommitment::from_commitments(&[commitment.clone()]),
        }
    }

    pub fn internal(left: &MerkleNode, right: &MerkleNode) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(left.hash);
        hasher.update(right.hash);
        Self {
            hash: hasher.finalize().into(),
            aggregate: left.aggregate.merge(&right.aggregate),
        }
    }
}

impl CommitmentMerkleTree {
    pub fn build(commitments: &[Commitment]) -> Result<Self> {
        if commitments.is_empty() {
            return Err(ArchimedesError::MerkleTreeError("Cannot build empty tree".to_string()));
        }
        let leaf_count = commitments.len();
        let leaves: Vec<MerkleNode> = commitments
            .iter()
            .enumerate()
            .map(|(i, c)| MerkleNode::leaf(c, i))
            .collect();
        let mut nodes = vec![leaves];
        while nodes.last().unwrap().len() > 1 {
            let prev_level = nodes.last().unwrap();
            let mut next_level = Vec::new();
            for chunk in prev_level.chunks(2) {
                if chunk.len() == 2 {
                    next_level.push(MerkleNode::internal(&chunk[0], &chunk[1]));
                } else {
                    next_level.push(chunk[0].clone());
                }
            }
            nodes.push(next_level);
        }
        Ok(Self { nodes, leaf_count })
    }

    pub fn root(&self) -> &MerkleNode {
        self.nodes.last().and_then(|l| l.first()).unwrap()
    }

    pub fn root_hash(&self) -> [u8; 32] {
        self.root().hash
    }

    pub fn aggregate(&self) -> &AggregateCommitment {
        &self.root().aggregate
    }

    pub fn range_aggregate(&self, start: usize, end: usize) -> Result<AggregateCommitment> {
        if end > self.leaf_count || start >= end {
            return Err(ArchimedesError::MerkleTreeError("Invalid range".to_string()));
        }
        let mut agg = AggregateCommitment::empty();
        for i in start..end {
            agg = agg.merge(&self.nodes[0][i].aggregate);
        }
        Ok(agg)
    }

    pub fn generate_proof(&self, index: usize) -> Result<MerkleProof> {
        if index >= self.leaf_count {
            return Err(ArchimedesError::MerkleTreeError("Index out of bounds".to_string()));
        }
        let mut siblings = Vec::new();
        let mut current_index = index;
        for level in 0..self.nodes.len() - 1 {
            let sibling_index = if current_index % 2 == 0 { current_index + 1 } else { current_index - 1 };
            if sibling_index < self.nodes[level].len() {
                siblings.push((self.nodes[level][sibling_index].hash, current_index % 2 == 0));
            }
            current_index /= 2;
        }
        Ok(MerkleProof { index, siblings })
    }

    pub fn leaf_count(&self) -> usize {
        self.leaf_count
    }
}

#[derive(Clone, Debug)]
pub struct MerkleProof {
    pub index: usize,
    pub siblings: Vec<([u8; 32], bool)>,
}

impl MerkleProof {
    pub fn verify(&self, leaf_hash: [u8; 32], root_hash: [u8; 32]) -> bool {
        let mut current = leaf_hash;
        for (sibling, is_left) in &self.siblings {
            let mut hasher = Sha256::new();
            if *is_left {
                hasher.update(current);
                hasher.update(sibling);
            } else {
                hasher.update(sibling);
                hasher.update(current);
            }
            current = hasher.finalize().into();
        }
        current == root_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use archimedes_core::{CommitmentChain, CommitmentParams};
    use ark_ed_on_bls12_381::Fr as ScalarField;
    use ark_std::test_rng;

    #[test]
    fn test_merkle_tree_build() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let mut chain = CommitmentChain::new(params);
        for i in 1..=8 {
            chain.push(ScalarField::from(i as u64), &mut rng).unwrap();
        }
        let tree = CommitmentMerkleTree::build(&chain.commitments).unwrap();
        assert_eq!(tree.leaf_count(), 8);
        assert_eq!(tree.aggregate().count, 8);
    }

    #[test]
    fn test_merkle_proof() {
        let mut rng = test_rng();
        let params = CommitmentParams::setup(&mut rng).unwrap();
        let mut chain = CommitmentChain::new(params);
        for i in 1..=4 {
            chain.push(ScalarField::from(i as u64), &mut rng).unwrap();
        }
        let tree = CommitmentMerkleTree::build(&chain.commitments).unwrap();
        let proof = tree.generate_proof(2).unwrap();
        let leaf_hash = tree.nodes[0][2].hash;
        assert!(proof.verify(leaf_hash, tree.root_hash()));
    }
}

