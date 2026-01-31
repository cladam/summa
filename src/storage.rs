//! Sled-based storage for summaries.

use crate::summary::Summary;
use serde_json;
use std::path::Path;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum StorageError {
    #[error("database error: {0}")]
    DbError(#[from] sled::Error),
    #[error("serialization error: {0}")]
    SerializationError(#[from] serde_json::Error),
    #[error("summary not found: {0}")]
    NotFound(String),
}

/// Sled-based storage for webpage summaries.
///
/// Stores summaries keyed by URL hash for efficient retrieval.
pub struct Storage {
    db: sled::Db,
}

impl Storage {
    /// Open or create storage at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, StorageError> {
        let db = sled::open(path)?;
        Ok(Self { db })
    }

    /// Store a summary for a URL
    pub fn store(&self, url: &str, summary: &Summary) -> Result<(), StorageError> {
        let key = Self::hash_url(url);
        let value = serde_json::to_vec(summary)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    /// Retrieve a summary by URL
    pub fn get(&self, url: &str) -> Result<Option<Summary>, StorageError> {
        let key = Self::hash_url(url);
        match self.db.get(key.as_bytes())? {
            Some(data) => {
                let summary: Summary = serde_json::from_slice(&data)?;
                Ok(Some(summary))
            }
            None => Ok(None),
        }
    }

    /// List all stored URLs and their summaries
    pub fn list_all(&self) -> Result<Vec<(String, Summary)>, StorageError> {
        let mut results = Vec::new();
        for item in self.db.iter() {
            let (key, value) = item?;
            let url = String::from_utf8_lossy(&key).to_string();
            let summary: Summary = serde_json::from_slice(&value)?;
            results.push((url, summary));
        }
        Ok(results)
    }

    /// Delete a summary by URL
    pub fn delete(&self, url: &str) -> Result<bool, StorageError> {
        let key = Self::hash_url(url);
        let existed = self.db.remove(key.as_bytes())?.is_some();
        self.db.flush()?;
        Ok(existed)
    }

    /// Create a hash of the URL for use as a key
    fn hash_url(url: &str) -> String {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};
        let mut hasher = DefaultHasher::new();
        url.hash(&mut hasher);
        format!("{:x}", hasher.finish())
    }
}
