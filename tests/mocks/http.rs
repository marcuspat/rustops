//! Mock HTTP server for testing.

use std::collections::HashMap;
use hyper::{Body, Request, Response, Server};
use hyper::service::{make_service_fn, service_fn};
use hyper::http::StatusCode;
use std::sync::{Arc, Mutex};
use tokio::sync::RwLock;

/// Mock HTTP response.
#[derive(Debug, Clone)]
pub struct MockResponse {
    pub status: StatusCode,
    pub body: String,
    pub headers: HashMap<String, String>,
}

impl MockResponse {
    /// Create a new mock response.
    pub fn new(status: StatusCode, body: impl Into<String>) -> Self {
        Self {
            status,
            body: body.into(),
            headers: HashMap::new(),
        }
    }

    /// Create a 200 OK response.
    pub fn ok(body: impl Into<String>) -> Self {
        Self::new(StatusCode::OK, body)
    }

    /// Create a 404 Not Found response.
    pub fn not_found() -> Self {
        Self::new(StatusCode::NOT_FOUND, "Not Found")
    }

    /// Create a 500 Internal Server Error response.
    pub fn internal_error() -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, "Internal Server Error")
    }

    /// Add a header.
    pub fn header(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(key.into(), value.into());
        self
    }
}

/// Mock HTTP server for testing.
pub struct MockHttpServer {
    responses: Arc<RwLock<HashMap<String, MockResponse>>>,
    port: u16,
}

impl MockHttpServer {
    /// Create a new mock HTTP server.
    pub fn new() -> Self {
        Self {
            responses: Arc::new(RwLock::new(HashMap::new())),
            port: 0,
        }
    }

    /// Set a response for a path.
    pub async fn set_response(&self, path: impl Into<String>, response: MockResponse) {
        let mut responses = self.responses.write().await;
        responses.insert(path.into(), response);
    }

    /// Get the server URL.
    pub fn url(&self) -> String {
        format!("http://127.0.0.1:{}", self.port)
    }

    /// Start the server.
    pub async fn start(&mut self) -> hyper::Result<()> {
        let responses = self.responses.clone();

        let make_svc = make_service_fn(move |_conn| {
            let responses = responses.clone();
            async move {
                Ok::<_, hyper::Error>(service_fn(move |req| {
                    let responses = responses.clone();
                    async move {
                        Self::handle_request(req, responses).await
                    }
                }))
            }
        });

        let addr = ([127, 0, 0, 1], 0).into();
        let server = Server::try_bind(&addr)?.serve(make_svc);

        self.port = server.local_addr().port();
        let _ = tokio::spawn(async move {
            server.await.unwrap();
        });

        // Give server time to start
        tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;

        Ok(())
    }

    async fn handle_request(
        req: Request<Body>,
        responses: Arc<RwLock<HashMap<String, MockResponse>>>,
    ) -> Result<Response<Body>, hyper::Error> {
        let path = req.uri().path().to_string();

        let responses = responses.read().await;
        let mock_response = responses.get(&path)
            .cloned()
            .unwrap_or_else(|| MockResponse::not_found());

        let mut builder = Response::builder()
            .status(mock_response.status);

        for (key, value) in mock_response.headers {
            builder = builder.header(key, value);
        }

        Ok(builder.body(Body::from(mock_response.body)).unwrap())
    }
}

impl Default for MockHttpServer {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_mock_http_server() {
        let mut server = MockHttpServer::new();
        server.start().await.unwrap();

        server.set_response(
            "/test",
            MockResponse::ok("Hello, World!")
        ).await;

        let client = reqwest::Client::new();
        let response = client.get(&format!("{}/test", server.url()))
            .send()
            .await
            .unwrap();

        assert_eq!(response.status(), 200);
        assert_eq!(response.text().await.unwrap(), "Hello, World!");
    }

    #[tokio::test]
    async fn test_mock_response_builder() {
        let response = MockResponse::ok("test")
            .header("Content-Type", "text/plain")
            .header("X-Custom", "value");

        assert_eq!(response.status, StatusCode::OK);
        assert_eq!(response.body, "test");
        assert_eq!(response.headers.len(), 2);
    }
}
