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
    /// Constructs a new `HttpClient` configured to negotiate HTTP/2 via ALPN.
    ///
    /// The created client uses a 30-second default timeout and is configured to bypass system proxy lookup.
    ///
    /// # Errors
    ///
    /// Returns a `PipError::Http` if building the underlying HTTP client fails.
    ///
    /// # Examples
    ///
    /// ```
    /// use piptable_http::HttpClient;
    /// let client = HttpClient::new().expect("failed to create HttpClient");
    /// ```
    pub fn new() -> PipResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(30))
            // Disable system proxy lookup to avoid macOS system-configuration issues
            .no_proxy()
            .build()
            .map_err(|e| PipError::Http(e.to_string()))?;

        Ok(Self { client })
    }

    /// Constructs an HttpClient configured with a custom per-request timeout.
    ///
    /// Builds an underlying reqwest client with the given timeout (in seconds) and disables proxy
    /// discovery for the process.
    ///
    /// # Errors
    ///
    /// Returns `PipError::Http` if building the underlying HTTP client fails.
    ///
    /// # Examples
    ///
    /// ```
    /// let client = piptable_http::HttpClient::with_timeout(10).unwrap();
    /// ```
    pub fn with_timeout(timeout_secs: u64) -> PipResult<Self> {
        let client = Client::builder()
            .timeout(Duration::from_secs(timeout_secs))
            .no_proxy()
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

    // ========================================================================
    // FetchOptions tests
    // ========================================================================

    #[test]
    fn test_fetch_options_default() {
        let opts = FetchOptions::default();
        assert!(matches!(opts.method, HttpMethod::Get));
        assert!(opts.headers.is_empty());
        assert!(opts.body.is_none());
        assert!(opts.timeout_secs.is_none());
    }

    #[test]
    fn test_fetch_options_with_values() {
        let mut headers = HashMap::new();
        headers.insert("Content-Type".to_string(), "application/json".to_string());

        let opts = FetchOptions {
            method: HttpMethod::Post,
            headers,
            body: Some("{\"key\": \"value\"}".to_string()),
            timeout_secs: Some(60),
        };

        assert!(matches!(opts.method, HttpMethod::Post));
        assert_eq!(opts.headers.len(), 1);
        assert!(opts.body.is_some());
        assert_eq!(opts.timeout_secs, Some(60));
    }

    // ========================================================================
    // HttpMethod tests
    // ========================================================================

    #[test]
    fn test_http_method_default() {
        let method = HttpMethod::default();
        assert!(matches!(method, HttpMethod::Get));
    }

    #[test]
    fn test_http_method_variants() {
        // Just ensure all variants are accessible
        let _ = HttpMethod::Get;
        let _ = HttpMethod::Post;
        let _ = HttpMethod::Put;
        let _ = HttpMethod::Delete;
        let _ = HttpMethod::Patch;
    }

    #[test]
    fn test_http_method_debug() {
        let method = HttpMethod::Get;
        let debug = format!("{:?}", method);
        assert_eq!(debug, "Get");
    }

    #[test]
    fn test_http_method_clone() {
        let method = HttpMethod::Post;
        let cloned = method.clone();
        assert!(matches!(cloned, HttpMethod::Post));
    }

    // ========================================================================
    // HttpClient construction tests
    // ========================================================================

    #[test]
    fn test_http_client_new() {
        let client = HttpClient::new();
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_client_with_timeout() {
        let client = HttpClient::with_timeout(10);
        assert!(client.is_ok());

        let client = HttpClient::with_timeout(120);
        assert!(client.is_ok());
    }

    #[test]
    fn test_http_client_default() {
        // Default impl should succeed
        let _client = HttpClient::default();
    }

    // ========================================================================
    // FetchOptions Clone/Debug tests
    // ========================================================================

    #[test]
    fn test_fetch_options_clone() {
        let opts = FetchOptions {
            method: HttpMethod::Put,
            headers: HashMap::new(),
            body: Some("test".to_string()),
            timeout_secs: Some(30),
        };
        let cloned = opts.clone();
        assert!(matches!(cloned.method, HttpMethod::Put));
        assert_eq!(cloned.body, Some("test".to_string()));
    }

    #[test]
    fn test_fetch_options_debug() {
        let opts = FetchOptions::default();
        let debug = format!("{:?}", opts);
        assert!(debug.contains("FetchOptions"));
    }
}
