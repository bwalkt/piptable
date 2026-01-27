//! Http tests for the PipTable interpreter.

#![allow(clippy::needless_raw_string_hashes)]

mod common;
use common::*;

use piptable_core::Value;
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Verifies that fetching a JSON array via HTTP produces an Array value in the interpreter.
///
/// # Examples
///
/// ```
/// // Starts a mock server that returns a JSON array at /api/items,
/// // then runs a PipTable script that fetches that endpoint and asserts the result is an array.
/// let server = wiremock::MockServer::start().await;
/// wiremock::Mock::given(wiremock::matchers::method("GET"))
///     .and(wiremock::matchers::path("/api/items"))
///     .respond_with(
///         wiremock::ResponseTemplate::new(200)
///             .set_body_json(serde_json::json!([
///                 {"id": 1, "name": "item1"},
///                 {"id": 2, "name": "item2"}
///             ])),
///     )
///     .mount(&server)
///     .await;
///
/// let script = format!(r#"dim data = fetch("{}/api/items")"#, server.uri());
/// let (interp, _) = run_script(&script).await;
///
/// match interp.get_var("data").await {
///     Some(Value::Array(items)) => assert_eq!(items.len(), 2),
///     _ => panic!("Expected array"),
/// }
/// ```
#[tokio::test]
async fn test_fetch_json_array() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/items"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"id": 1, "name": "item1"},
            {"id": 2, "name": "item2"}
        ])))
        .mount(&server)
        .await;

    let script = format!(r#"dim data = fetch("{}/api/items")"#, server.uri());
    let (interp, _) = run_script(&script).await;

    match interp.get_var("data").await {
        Some(Value::Array(items)) => {
            assert_eq!(items.len(), 2);
        }
        _ => panic!("Expected array"),
    }
}

/// Verifies that an HTTP GET returning a JSON object is parsed into an interpreter `Object` value.
///
/// The test starts a mock HTTP server that responds with a JSON object, runs a script that calls
/// `fetch` on that endpoint, and asserts the interpreter binds an object whose `"name"` field equals
/// `"test"`.
///
/// # Examples
///
/// ```
/// // Starts a mock server responding with {"id":1,"name":"test"} and ensures the interpreter's
/// // `data` variable is an object containing `name = "test"`.
/// # async fn _example() { /* runs within test harness */ }
/// ```
#[tokio::test]
async fn test_fetch_json_object() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/user"))
        .respond_with(
            ResponseTemplate::new(200).set_body_json(serde_json::json!({"id": 1, "name": "test"})),
        )
        .mount(&server)
        .await;

    let script = format!(r#"dim data = fetch("{}/api/user")"#, server.uri());
    let (interp, _) = run_script(&script).await;

    match interp.get_var("data").await {
        Some(Value::Object(map)) => {
            assert!(matches!(map.get("name"), Some(Value::String(s)) if s == "test"));
        }
        _ => panic!("Expected object"),
    }
}

/// Verifies that fetching a JSON array produces an iterable that can be traversed with `for each` and that iteration yields the expected aggregated result.
///
/// The test starts a mock HTTP server returning the JSON array `[1, 2, 3, 4, 5]`, runs a PipTable script that fetches that array, iterates over its elements to compute a running sum, and asserts the interpreter's `sum` variable equals `15`.
///
/// # Examples
///
/// ```no_run
/// // Starts a mock server returning [1,2,3,4,5], runs a script that fetches and sums the array,
/// // and then verifies the interpreter's `sum` variable is 15.
/// ```
#[tokio::test]
async fn test_fetch_and_iterate() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/api/numbers"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([1, 2, 3, 4, 5])))
        .mount(&server)
        .await;

    let script = format!(
        r#"
        dim data = fetch("{}/api/numbers")
        dim sum = 0
        for each n in data
            sum = sum + n
        next
    "#,
        server.uri()
    );
    let (interp, _) = run_script(&script).await;

    assert!(matches!(interp.get_var("sum").await, Some(Value::Int(15))));
}
