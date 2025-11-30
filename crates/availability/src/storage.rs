use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::HashMap;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("Content not found: {0}")]
    NotFound(String),
    #[error("Invalid content hash")]
    InvalidHash,
    #[error("Storage full")]
    StorageFull,
}

type Result<T> = std::result::Result<T, StorageError>;

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ContentId(pub [u8; 32]);

impl ContentId {
    pub fn from_data(data: &[u8]) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(data);
        let result = hasher.finalize();
        let mut id = [0u8; 32];
        id.copy_from_slice(&result);
        ContentId(id)
    }

    pub fn to_hex(&self) -> String {
        self.0.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct StoredContent {
    pub id: ContentId,
    pub data: Vec<u8>,
    pub timestamp: u64,
    pub reference_count: u32,
}

pub struct ContentAddressedStorage {
    store: HashMap<ContentId, StoredContent>,
    max_size: usize,
    current_size: usize,
}

impl ContentAddressedStorage {
    pub fn new(max_size: usize) -> Self {
        Self {
            store: HashMap::new(),
            max_size,
            current_size: 0,
        }
    }

    pub fn store(&mut self, data: Vec<u8>, timestamp: u64) -> Result<ContentId> {
        let id = ContentId::from_data(&data);
        
        if self.current_size + data.len() > self.max_size {
            return Err(StorageError::StorageFull);
        }

        if let Some(content) = self.store.get_mut(&id) {
            content.reference_count += 1;
            return Ok(id);
        }

        let size = data.len();
        let content = StoredContent {
            id: id.clone(),
            data,
            timestamp,
            reference_count: 1,
        };

        self.store.insert(id.clone(), content);
        self.current_size += size;
        Ok(id)
    }

    pub fn retrieve(&self, id: &ContentId) -> Result<&[u8]> {
        self.store.get(id)
            .map(|c| c.data.as_slice())
            .ok_or_else(|| StorageError::NotFound(id.to_hex()))
    }

    pub fn exists(&self, id: &ContentId) -> bool {
        self.store.contains_key(id)
    }

    pub fn remove(&mut self, id: &ContentId) -> Result<()> {
        if let Some(content) = self.store.get_mut(id) {
            content.reference_count = content.reference_count.saturating_sub(1);
            if content.reference_count == 0 {
                let size = content.data.len();
                self.store.remove(id);
                self.current_size -= size;
            }
        }
        Ok(())
    }

    pub fn size(&self) -> usize {
        self.current_size
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_store_and_retrieve() {
        let mut storage = ContentAddressedStorage::new(1024 * 1024);
        let data = b"hello world".to_vec();
        
        let id = storage.store(data.clone(), 100).unwrap();
        let retrieved = storage.retrieve(&id).unwrap();
        
        assert_eq!(retrieved, data.as_slice());
    }

    #[test]
    fn test_content_addressing() {
        let mut storage = ContentAddressedStorage::new(1024 * 1024);
        let data = b"same content".to_vec();
        
        let id1 = storage.store(data.clone(), 100).unwrap();
        let id2 = storage.store(data.clone(), 200).unwrap();
        
        assert_eq!(id1, id2);
    }

    #[test]
    fn test_storage_limit() {
        let mut storage = ContentAddressedStorage::new(10);
        let data = b"too much data".to_vec();
        
        let result = storage.store(data, 100);
        assert!(matches!(result, Err(StorageError::StorageFull)));
    }
}

