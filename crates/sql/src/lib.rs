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

    /// Deregister (drop) a table from the context.
    pub async fn deregister_table(&self, name: &str) -> PipResult<()> {
        self.ctx
            .deregister_table(name)
            .map_err(|e| PipError::Sql(format!("Failed to deregister table {}: {}", name, e)))?;
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
    use arrow::array::{Int64Array, StringArray};
    use arrow::datatypes::{DataType, Field, Schema};
    use std::io::Write;
    use tempfile::NamedTempFile;

    /// Helper to create a test RecordBatch
    fn create_test_batch() -> RecordBatch {
        let schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]);
        let id_array = Int64Array::from(vec![1, 2, 3]);
        let name_array = StringArray::from(vec!["alice", "bob", "charlie"]);
        RecordBatch::try_new(
            std::sync::Arc::new(schema),
            vec![
                std::sync::Arc::new(id_array),
                std::sync::Arc::new(name_array),
            ],
        )
        .unwrap()
    }

    /// Helper to create a temp CSV file
    fn create_temp_csv(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    /// Helper to create a temp JSON file (newline-delimited)
    fn create_temp_json(content: &str) -> NamedTempFile {
        let mut file = NamedTempFile::new().unwrap();
        file.write_all(content.as_bytes()).unwrap();
        file.flush().unwrap();
        file
    }

    // ===== Constructor Tests =====

    #[tokio::test]
    async fn test_new() {
        let engine = SqlEngine::new();
        // Verify engine works by running a simple query
        let result = engine.query("SELECT 1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_default() {
        let engine = SqlEngine::default();
        let result = engine.query("SELECT 1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_with_config() {
        let config = SessionConfig::new().with_batch_size(100);
        let engine = SqlEngine::with_config(config);
        let result = engine.query("SELECT 1").await;
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_context() {
        let engine = SqlEngine::new();
        let ctx = engine.context();
        // Verify we can use the context directly
        let df = ctx.sql("SELECT 1 as value").await.unwrap();
        let batches = df.collect().await.unwrap();
        assert_eq!(batches.len(), 1);
    }

    // ===== Query Tests =====

    #[tokio::test]
    async fn test_simple_query() {
        let engine = SqlEngine::new();
        let result = engine.query("SELECT 1 + 1 as result").await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        assert_eq!(batches.len(), 1);
        assert_eq!(batches[0].num_rows(), 1);
    }

    #[tokio::test]
    async fn test_query_multiple_rows() {
        let engine = SqlEngine::new();
        let result = engine
            .query("SELECT * FROM (VALUES (1, 'a'), (2, 'b'), (3, 'c')) as t(id, name)")
            .await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3);
    }

    #[tokio::test]
    async fn test_query_with_aggregation() {
        let engine = SqlEngine::new();
        let result = engine
            .query("SELECT COUNT(*) as cnt FROM (VALUES (1), (2), (3)) as t(x)")
            .await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        assert_eq!(batches[0].num_rows(), 1);
    }

    #[tokio::test]
    async fn test_query_df() {
        let engine = SqlEngine::new();
        let result = engine.query_df("SELECT 1 as value, 'test' as name").await;
        assert!(result.is_ok());
        let df = result.unwrap();
        let batches = df.collect().await.unwrap();
        assert_eq!(batches.len(), 1);
    }

    #[tokio::test]
    async fn test_query_invalid_sql() {
        let engine = SqlEngine::new();
        let result = engine.query("SELEC invalid syntax").await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_query_nonexistent_table() {
        let engine = SqlEngine::new();
        let result = engine.query("SELECT * FROM nonexistent_table").await;
        assert!(result.is_err());
    }

    // ===== Register Table Tests =====

    #[tokio::test]
    async fn test_register_table() {
        let engine = SqlEngine::new();
        let batch = create_test_batch();

        let result = engine.register_table("users", vec![batch]).await;
        assert!(result.is_ok());

        // Query the registered table
        let query_result = engine.query("SELECT * FROM users").await;
        assert!(query_result.is_ok());
        let batches = query_result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3);
    }

    #[tokio::test]
    async fn test_register_table_and_query() {
        let engine = SqlEngine::new();
        let batch = create_test_batch();

        engine.register_table("users", vec![batch]).await.unwrap();

        // Query with WHERE clause
        let result = engine.query("SELECT name FROM users WHERE id > 1").await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2); // bob and charlie
    }

    #[tokio::test]
    async fn test_register_table_empty() {
        let engine = SqlEngine::new();
        let result = engine.register_table("empty", vec![]).await;
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(err.to_string().contains("empty"));
    }

    #[tokio::test]
    async fn test_register_multiple_tables() {
        let engine = SqlEngine::new();
        let batch1 = create_test_batch();
        let batch2 = create_test_batch();

        engine.register_table("table1", vec![batch1]).await.unwrap();
        engine.register_table("table2", vec![batch2]).await.unwrap();

        // Join the tables
        let result = engine
            .query("SELECT t1.id, t2.name FROM table1 t1 JOIN table2 t2 ON t1.id = t2.id")
            .await;
        assert!(result.is_ok());
    }

    // ===== Register CSV Tests =====

    #[tokio::test]
    async fn test_register_csv() {
        let csv_content = "id,name,value\n1,foo,100\n2,bar,200\n3,baz,300";
        let file = create_temp_csv(csv_content);
        let path = file.path().to_string_lossy().to_string();

        let engine = SqlEngine::new();
        let result = engine.register_csv("data", &path).await;
        assert!(result.is_ok());

        // Query the CSV - file must remain open
        let query_result = engine.query("SELECT COUNT(*) as cnt FROM data").await;
        assert!(query_result.is_ok());
    }

    #[tokio::test]
    async fn test_register_csv_nonexistent() {
        let engine = SqlEngine::new();
        // Use OS-agnostic path that doesn't exist
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.csv");
        let result = engine
            .register_csv("data", nonexistent_path.to_str().unwrap())
            .await;
        assert!(result.is_err());
    }

    // ===== Register JSON Tests =====

    #[tokio::test]
    async fn test_register_json() {
        let json_content = r#"{"id": 1, "name": "alice"}
{"id": 2, "name": "bob"}
{"id": 3, "name": "charlie"}"#;
        let file = create_temp_json(json_content);
        let path = file.path().to_string_lossy().to_string();

        let engine = SqlEngine::new();
        let result = engine.register_json("users", &path).await;
        assert!(result.is_ok());

        // Just verify registration worked
        let query_result = engine.query("SELECT COUNT(*) as cnt FROM users").await;
        assert!(query_result.is_ok());
    }

    #[tokio::test]
    async fn test_register_json_nonexistent() {
        let engine = SqlEngine::new();
        // Use OS-agnostic path that doesn't exist
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.json");
        let result = engine
            .register_json("data", nonexistent_path.to_str().unwrap())
            .await;
        assert!(result.is_err());
    }

    // ===== Register Parquet Tests =====

    #[tokio::test]
    async fn test_register_parquet_nonexistent() {
        let engine = SqlEngine::new();
        // Use OS-agnostic path that doesn't exist
        let temp_dir = tempfile::tempdir().unwrap();
        let nonexistent_path = temp_dir.path().join("nonexistent.parquet");
        let result = engine
            .register_parquet("data", nonexistent_path.to_str().unwrap())
            .await;
        assert!(result.is_err());
    }

    // ===== Complex Query Tests with RecordBatch =====

    #[tokio::test]
    async fn test_query_with_where() {
        let engine = SqlEngine::new();
        let batch = create_test_batch();
        engine.register_table("users", vec![batch]).await.unwrap();

        let result = engine.query("SELECT name FROM users WHERE id > 1").await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2); // bob and charlie
    }

    #[tokio::test]
    async fn test_query_with_order_by() {
        let engine = SqlEngine::new();
        let batch = create_test_batch();
        engine.register_table("users", vec![batch]).await.unwrap();

        let result = engine.query("SELECT * FROM users ORDER BY id DESC").await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3);
    }

    #[tokio::test]
    async fn test_query_with_limit() {
        let engine = SqlEngine::new();
        let batch = create_test_batch();
        engine.register_table("users", vec![batch]).await.unwrap();

        let result = engine.query("SELECT * FROM users LIMIT 2").await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2);
    }

    #[tokio::test]
    async fn test_query_with_group_by() {
        // Create batch with duplicates for grouping
        let schema = Schema::new(vec![
            Field::new("category", DataType::Utf8, false),
            Field::new("amount", DataType::Int64, false),
        ]);
        let cat_array = StringArray::from(vec!["A", "B", "A", "B"]);
        let amt_array = Int64Array::from(vec![100, 200, 150, 50]);
        let batch = RecordBatch::try_new(
            std::sync::Arc::new(schema),
            vec![
                std::sync::Arc::new(cat_array),
                std::sync::Arc::new(amt_array),
            ],
        )
        .unwrap();

        let engine = SqlEngine::new();
        engine.register_table("sales", vec![batch]).await.unwrap();

        let result = engine
            .query("SELECT category, SUM(amount) as total FROM sales GROUP BY category ORDER BY category")
            .await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 2); // A and B
    }

    #[tokio::test]
    async fn test_table_join() {
        let engine = SqlEngine::new();

        // Users table
        let users_schema = Schema::new(vec![
            Field::new("id", DataType::Int64, false),
            Field::new("name", DataType::Utf8, false),
        ]);
        let users_batch = RecordBatch::try_new(
            std::sync::Arc::new(users_schema),
            vec![
                std::sync::Arc::new(Int64Array::from(vec![1, 2])),
                std::sync::Arc::new(StringArray::from(vec!["alice", "bob"])),
            ],
        )
        .unwrap();

        // Orders table
        let orders_schema = Schema::new(vec![
            Field::new("user_id", DataType::Int64, false),
            Field::new("amount", DataType::Int64, false),
        ]);
        let orders_batch = RecordBatch::try_new(
            std::sync::Arc::new(orders_schema),
            vec![
                std::sync::Arc::new(Int64Array::from(vec![1, 1, 2])),
                std::sync::Arc::new(Int64Array::from(vec![100, 200, 50])),
            ],
        )
        .unwrap();

        engine
            .register_table("users", vec![users_batch])
            .await
            .unwrap();
        engine
            .register_table("orders", vec![orders_batch])
            .await
            .unwrap();

        let result = engine
            .query("SELECT u.name, o.amount FROM users u JOIN orders o ON u.id = o.user_id")
            .await;
        assert!(result.is_ok());
        let batches = result.unwrap();
        let total_rows: usize = batches.iter().map(|b| b.num_rows()).sum();
        assert_eq!(total_rows, 3);
    }
}
