//! Tantivy-based full-text search index.

use crate::summary::Summary;
use std::path::Path;
use tantivy::collector::TopDocs;
use tantivy::query::QueryParser;
use tantivy::schema::{Schema, Value, STORED, TEXT};
use tantivy::{doc, Index, IndexWriter, ReloadPolicy};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum SearchError {
    #[error("index error: {0}")]
    IndexError(#[from] tantivy::TantivyError),
    #[error("query parse error: {0}")]
    QueryError(#[from] tantivy::query::QueryParserError),
    #[error("io error: {0}")]
    IoError(#[from] std::io::Error),
}

/// Tantivy-based search index for summaries.
pub struct SearchIndex {
    index: Index,
    schema: Schema,
}

impl SearchIndex {
    /// Open or create a search index at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, SearchError> {
        let mut schema_builder = Schema::builder();
        schema_builder.add_text_field("url", TEXT | STORED);
        schema_builder.add_text_field("title", TEXT | STORED);
        schema_builder.add_text_field("conclusion", TEXT);
        schema_builder.add_text_field("key_points", TEXT);
        schema_builder.add_text_field("entities", TEXT);
        schema_builder.add_text_field("action_items", TEXT);
        let schema = schema_builder.build();

        let index_path = path.as_ref();
        std::fs::create_dir_all(index_path)?;

        let index = Index::create_in_dir(index_path, schema.clone())
            .or_else(|_| Index::open_in_dir(index_path))?;

        Ok(Self { index, schema })
    }

    /// Index a summary for searching
    pub fn index_summary(&self, url: &str, summary: &Summary) -> Result<(), SearchError> {
        let mut index_writer: IndexWriter = self.index.writer(50_000_000)?;

        let url_field = self.schema.get_field("url").unwrap();
        let title_field = self.schema.get_field("title").unwrap();
        let conclusion_field = self.schema.get_field("conclusion").unwrap();
        let key_points_field = self.schema.get_field("key_points").unwrap();
        let entities_field = self.schema.get_field("entities").unwrap();
        let action_items_field = self.schema.get_field("action_items").unwrap();

        // Delete any existing document with this URL first
        let url_term = tantivy::Term::from_field_text(url_field, url);
        index_writer.delete_term(url_term);

        index_writer.add_document(doc!(
            url_field => url,
            title_field => summary.title.clone(),
            conclusion_field => summary.conclusion.clone(),
            key_points_field => summary.key_points.join(" "),
            entities_field => summary.entities.join(" "),
            action_items_field => summary.action_items.join(" "),
        ))?;

        index_writer.commit()?;
        Ok(())
    }

    /// Search for summaries matching the query
    pub fn search(&self, query_str: &str, limit: usize) -> Result<Vec<String>, SearchError> {
        let reader = self
            .index
            .reader_builder()
            .reload_policy(ReloadPolicy::OnCommitWithDelay)
            .try_into()?;

        let searcher = reader.searcher();
        let title_field = self.schema.get_field("title").unwrap();
        let conclusion_field = self.schema.get_field("conclusion").unwrap();
        let key_points_field = self.schema.get_field("key_points").unwrap();
        let entities_field = self.schema.get_field("entities").unwrap();

        let query_parser = QueryParser::for_index(
            &self.index,
            vec![
                title_field,
                conclusion_field,
                key_points_field,
                entities_field,
            ],
        );
        let query = query_parser.parse_query(query_str)?;

        let top_docs = searcher.search(&query, &TopDocs::with_limit(limit))?;

        let url_field = self.schema.get_field("url").unwrap();
        let mut results = Vec::new();
        for (_score, doc_address) in top_docs {
            let retrieved_doc = searcher.doc::<tantivy::TantivyDocument>(doc_address)?;
            if let Some(url) = retrieved_doc.get_first(url_field) {
                if let Some(url_str) = url.as_str() {
                    results.push(url_str.to_string());
                }
            }
        }

        Ok(results)
    }
}
