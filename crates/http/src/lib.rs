//! # piptable-http
//!
//! HTTP client for fetching data from APIs.
//!
//! This crate provides async HTTP fetching with JSON parsing.
//! Supports HTTP/2 via ALPN negotiation with fallback to HTTP/1.1.

use piptable_core::{PipError, PipResult, Value};
use reqwest::Client;
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::time::Duration;

/// HTTP client for data fetching.
pub struct HttpClient {
    client: Client,
}

/// Options for HTTP requests.
#[derive(Debug, Clone, Default)]
pub struct FetchOptions {
    pub method: HttpMethod,
    pub headers: HashMap<String, String>,
    pub body: Option<String>,
    pub timeout_secs: Option<u64>,
}

/// HTTP methods.
#[derive(Debug, Clone, Default)]
pub enum HttpMethod {
    #[default]
    Get,
    Post,
    Put,
    Delete,
    Patch,
}

impl HttpClient {
    /// Create a new HTTP client with HTTP/2 support via ALPN negotiation.
    ///
    /// # Errors
    ///
    /// Returns error if client creation fails.
    pub fn new() -> PipResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| PipError::Http(e.to_string()))?;

        Ok(Self { client })
    }

    /// Create with custom configuration.
    ///
    /// # Errors
    ///
    /// Returns error if client creation fails.
    pub fn with_timeout(timeout_secs: u64) -> PipResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .build()
            .map_err(|e| PipError::Http(e.to_string()))?;

        Ok(Self { client })
    }

    /// Fetch data from a URL.
    ///
    /// # Errors
    ///
    /// Returns error if request fails or response cannot be parsed.
    pub async fn fetch(&self, url: &str, options: Option<FetchOptions>) -> PipResult<Value> {
        let opts = options.unwrap_or_default();

        let mut request = match opts.method {
            HttpMethod::Get => self.client.get(url),
            HttpMethod::Post => self.client.post(url),
            HttpMethod::Put => self.client.put(url),
            HttpMethod::Delete => self.client.delete(url),
            HttpMethod::Patch => self.client.patch(url),
        };

        // Add headers
        for (key, value) in &opts.headers {
            request = request.header(key, value);
        }

        // Add body if present
        if let Some(body) = opts.body {
            request = request.body(body);
        }

        // Set timeout if specified
        if let Some(timeout) = opts.timeout_secs {
            request = request.timeout(Duration::from_secs(timeout));
        }

        let response = request
            .send()
            .await
            .map_err(|e| PipError::Http(e.to_string()))?;

        if !response.status().is_success() {
            return Err(PipError::Http(format!(
                "HTTP {} - {}",
                response.status(),
                response.status().canonical_reason().unwrap_or("Unknown")
            )));
        }

        let json: JsonValue = response
            .json()
            .await
            .map_err(|e| PipError::Http(format!("Failed to parse JSON: {e}")))?;

        Ok(Value::from_json(json))
    }

    /// Fetch multiple URLs concurrently.
    ///
    /// # Errors
    ///
    /// Returns error if any request fails.
    pub async fn fetch_all(&self, urls: Vec<&str>) -> PipResult<Vec<Value>> {
        let futures: Vec<_> = urls.iter().map(|url| self.fetch(url, None)).collect();

        let results = futures::future::join_all(futures).await;

        results.into_iter().collect()
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new().expect("Failed to create HTTP client")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fetch_options_default() {
        let opts = FetchOptions::default();
        assert!(matches!(opts.method, HttpMethod::Get));
        assert!(opts.headers.is_empty());
        assert!(opts.body.is_none());
    }
}
