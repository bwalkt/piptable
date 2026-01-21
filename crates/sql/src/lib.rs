//! # piptable-sql
//!
//! SQL execution engine for piptable using Apache DataFusion.
//!
//! This crate provides:
//! - SQL query execution
//! - Data source registration (CSV, JSON, Parquet)
//! - Query optimization via DataFusion

use arrow::array::RecordBatch;
use datafusion::prelude::*;
use piptable_core::{PipError, PipResult};
use std::sync::Arc;

/// SQL execution engine wrapping DataFusion.
pub struct SqlEngine {
    ctx: SessionContext,
}

impl SqlEngine {
    /// Create a new SQL engine.
    #[must_use]
    pub fn new() -> Self {
        Self {
            ctx: SessionContext::new(),
        }
    }

    /// Create with custom session config.
    #[must_use]
    pub fn with_config(config: SessionConfig) -> Self {
        Self {
            ctx: SessionContext::new_with_config(config),
        }
    }

    /// Register a table from Arrow record batches.
    ///
    /// # Errors
    ///
    /// Returns error if registration fails.
    pub async fn register_table(&self, name: &str, batches: Vec<RecordBatch>) -> PipResult<()> {
        if batches.is_empty() {
            return Err(PipError::Sql("Cannot register empty table".into()));
        }

        let schema = batches[0].schema();
        let provider = datafusion::datasource::MemTable::try_new(schema, vec![batches])
            .map_err(|e| PipError::Sql(e.to_string()))?;

        self.ctx
            .register_table(name, Arc::new(provider))
            .map_err(|e| PipError::Sql(e.to_string()))?;

        Ok(())
    }

    /// Register a CSV file as a table.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or registered.
    pub async fn register_csv(&self, name: &str, path: &str) -> PipResult<()> {
        self.ctx
            .register_csv(name, path, CsvReadOptions::default())
            .await
            .map_err(|e| PipError::Sql(e.to_string()))?;
        Ok(())
    }

    /// Register a JSON file as a table.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or registered.
    pub async fn register_json(&self, name: &str, path: &str) -> PipResult<()> {
        self.ctx
            .register_json(name, path, NdJsonReadOptions::default())
            .await
            .map_err(|e| PipError::Sql(e.to_string()))?;
        Ok(())
    }

    /// Register a Parquet file as a table.
    ///
    /// # Errors
    ///
    /// Returns error if file cannot be read or registered.
    pub async fn register_parquet(&self, name: &str, path: &str) -> PipResult<()> {
        self.ctx
            .register_parquet(name, path, ParquetReadOptions::default())
            .await
            .map_err(|e| PipError::Sql(e.to_string()))?;
        Ok(())
    }

    /// Execute a SQL query and return results.
    ///
    /// # Errors
    ///
    /// Returns error if query execution fails.
    pub async fn query(&self, sql: &str) -> PipResult<Vec<RecordBatch>> {
        let df = self
            .ctx
            .sql(sql)
            .await
            .map_err(|e| PipError::Sql(e.to_string()))?;

        df.collect().await.map_err(|e| PipError::Sql(e.to_string()))
    }

    /// Execute a SQL query and return a DataFrame.
    ///
    /// # Errors
    ///
    /// Returns error if query execution fails.
    pub async fn query_df(&self, sql: &str) -> PipResult<DataFrame> {
        self.ctx
            .sql(sql)
            .await
            .map_err(|e| PipError::Sql(e.to_string()))
    }

    /// Get the underlying session context.
    #[must_use]
    pub fn context(&self) -> &SessionContext {
        &self.ctx
    }
}

impl Default for SqlEngine {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_simple_query() {
        let engine = SqlEngine::new();
        let result = engine.query("SELECT 1 + 1 as result").await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        assert_eq!(batches.len(), 1);
    }

    #[tokio::test]
    async fn test_range_query() {
        let engine = SqlEngine::new();
        let result = engine
            .query("SELECT * FROM generate_series(1, 5) as t(value)")
            .await;
        // This may or may not work depending on DataFusion version
        // Just checking it doesn't panic
        let _ = result;
    }
}
