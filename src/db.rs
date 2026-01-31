//! Database module for storage and search.
//!
//! Uses sled for K/V storage and tantivy for full-text search.

pub use crate::storage::Storage;
pub use crate::search::SearchIndex;
