//! Sled-based storage for summaries.

use crate::summary::Summary;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
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

/// A stored summary with metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StoredSummary {
    /// The source URL
    pub url: String,
    /// When the summary was created
    pub created_at: DateTime<Utc>,
    /// The summary itself
    pub summary: Summary,
}

impl StoredSummary {
    /// Create a new stored summary
    pub fn new(url: String, summary: Summary) -> Self {
        Self {
            url,
            created_at: Utc::now(),
            summary,
        }
    }
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
        let stored = StoredSummary::new(url.to_string(), summary.clone());
        let value = serde_json::to_vec(&stored)?;
        self.db.insert(key.as_bytes(), value)?;
        self.db.flush()?;
        Ok(())
    }

    /// Retrieve a summary by URL
    pub fn get(&self, url: &str) -> Result<Option<StoredSummary>, StorageError> {
        let key = Self::hash_url(url);
        match self.db.get(key.as_bytes())? {
            Some(data) => {
                let stored: StoredSummary = serde_json::from_slice(&data)?;
                Ok(Some(stored))
            }
            None => Ok(None),
        }
    }

    /// List all stored summaries
    pub fn list_all(&self) -> Result<Vec<StoredSummary>, StorageError> {
        let mut results = Vec::new();
        for item in self.db.iter() {
            let (_key, value) = item?;
            let stored: StoredSummary = serde_json::from_slice(&value)?;
            results.push(stored);
        }
        // Sort by created_at descending (newest first)
        results.sort_by(|a, b| b.created_at.cmp(&a.created_at));
        Ok(results)
    }

    /// Delete a summary by URL
    pub fn delete(&self, url: &str) -> Result<bool, StorageError> {
        let key = Self::hash_url(url);
        let existed = self.db.remove(key.as_bytes())?.is_some();
        self.db.flush()?;
        Ok(existed)
    }

    /// Get the number of stored summaries
    pub fn count(&self) -> usize {
        self.db.len()
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
