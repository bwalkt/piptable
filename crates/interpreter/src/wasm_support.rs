use arrow::array::RecordBatch;
use piptable_core::{PipError, PipResult, Value};

pub struct SqlEngine;

impl SqlEngine {
    #[must_use]
    pub fn new() -> Self {
        Self
    }

    pub async fn register_table(&self, _name: &str, _batches: Vec<RecordBatch>) -> PipResult<()> {
        Err(PipError::Sql("SQL is not supported in the playground".into()))
    }

    pub async fn register_csv(&self, _name: &str, _path: &str) -> PipResult<()> {
        Err(PipError::Sql("SQL is not supported in the playground".into()))
    }

    pub async fn register_json(&self, _name: &str, _path: &str) -> PipResult<()> {
        Err(PipError::Sql("SQL is not supported in the playground".into()))
    }

    pub async fn register_parquet(&self, _name: &str, _path: &str) -> PipResult<()> {
        Err(PipError::Sql("SQL is not supported in the playground".into()))
    }

    pub async fn deregister_table(&self, _name: &str) -> PipResult<()> {
        Ok(())
    }

    pub async fn query(&self, _sql: &str) -> PipResult<Vec<RecordBatch>> {
        Err(PipError::Sql("SQL is not supported in the playground".into()))
    }
}

pub struct HttpClient;

#[derive(Debug, Clone, Default)]
pub struct FetchOptions;

impl HttpClient {
    pub fn new() -> PipResult<Self> {
        Ok(Self)
    }

    pub async fn fetch(&self, _url: &str, _options: Option<FetchOptions>) -> PipResult<Value> {
        Err(PipError::Http(
            "HTTP fetch is not supported in the playground".into(),
        ))
    }
}
