//! # Error Types
//!
//! Custom error types for the topology bounded context.

use rustops_common::Error as CommonError;
use thiserror::Error;

/// Topology result type
pub type Result<T> = std::result::Result<T, Error>;

/// Topology error types
#[derive(Debug, Error)]
pub enum Error {
    /// Graph operation error
    #[error("Graph operation failed: {0}")]
    Graph(String),

    /// Discovery error
    #[error("Discovery failed: {0}")]
    Discovery(String),

    /// Storage error
    #[error("Storage operation failed: {0}")]
    Storage(String),

    /// Validation error
    #[error("Validation failed: {0}")]
    Validation(String),

    /// Configuration error
    #[error("Configuration error: {0}")]
    Config(String),

    /// Network error
    #[error("Network error: {0}")]
    Network(String),

    /// Kubernetes API error
    #[error("Kubernetes API error: {0}")]
    Kubernetes(String),

    /// Neo4j database error
    #[error("Graph database error: {0}")]
    GraphDatabase(String),

    /// Event store error
    #[error("Event store error: {0}")]
    EventStore(String),

    /// Impact analysis error
    #[error("Impact analysis error: {0}")]
    ImpactAnalysis(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO error
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON error
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    /// Anyhow error
    #[error("Anyhow error: {0}")]
    Anyhow(#[from] anyhow::Error),

    /// HTTP error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Timeout error
    #[error("Operation timed out: {0}")]
    Timeout(String),

    /// Permission error
    #[error("Permission denied: {0}")]
    Permission(String),

    /// Service not found error
    #[error("Service not found: {0}")]
    ServiceNotFound(String),

    /// Dependency not found error
    #[error("Dependency not found: {0}")]
    DependencyNotFound(String),

    /// Conflict error
    #[error("Conflict: {0}")]
    Conflict(String),

    /// Rate limit error
    #[error("Rate limit exceeded: {0}")]
    RateLimit(String),

    /// Authentication error
    #[error("Authentication failed: {0}")]
    Authentication(String),

    /// Authorization error
    #[error("Authorization failed: {0}")]
    Authorization(String),

    /// Database connection error
    #[error("Database connection failed: {0}")]
    Connection(String),

    /// Transaction error
    #[error("Transaction failed: {0}")]
    Transaction(String),

    /// Query error
    #[error("Query failed: {0}")]
    Query(String),

    /// Invalid state error
    #[error("Invalid state: {0}")]
    InvalidState(String),

    /// Unsupported operation error
    #[error("Unsupported operation: {0}")]
    Unsupported(String),

    /// Resource not found error
    #[error("Resource not found: {0}")]
    ResourceNotFound(String),

    /// Resource already exists error
    #[error("Resource already exists: {0}")]
    ResourceExists(String),

    /// Internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a new graph error
    pub fn graph(msg: impl Into<String>) -> Self {
        Error::Graph(msg.into())
    }

    /// Create a new discovery error
    pub fn discovery(msg: impl Into<String>) -> Self {
        Error::Discovery(msg.into())
    }

    /// Create a new storage error
    pub fn storage(msg: impl Into<String>) -> Self {
        Error::Storage(msg.into())
    }

    /// Create a new validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Error::Validation(msg.into())
    }

    /// Create a new configuration error
    pub fn config(msg: impl Into<String>) -> Self {
        Error::Config(msg.into())
    }

    /// Create a new network error
    pub fn network(msg: impl Into<String>) -> Self {
        Error::Network(msg.into())
    }

    /// Create a new Kubernetes API error
    pub fn kubernetes(msg: impl Into<String>) -> Self {
        Error::Kubernetes(msg.into())
    }

    /// Create a new Neo4j database error
    pub fn graph_database(msg: impl Into<String>) -> Self {
        Error::GraphDatabase(msg.into())
    }

    /// Create a new event store error
    pub fn event_store(msg: impl Into<String>) -> Self {
        Error::EventStore(msg.into())
    }

    /// Create a new impact analysis error
    pub fn impact_analysis(msg: impl Into<String>) -> Self {
        Error::ImpactAnalysis(msg.into())
    }

    /// Create a new serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Error::Serialization(msg.into())
    }

    /// Create a new HTTP error
    pub fn http(msg: impl Into<String>) -> Self {
        Error::Http(msg.into())
    }

    /// Create a new timeout error
    pub fn timeout(msg: impl Into<String>) -> Self {
        Error::Timeout(msg.into())
    }

    /// Create a new permission error
    pub fn permission(msg: impl Into<String>) -> Self {
        Error::Permission(msg.into())
    }

    /// Create a new service not found error
    pub fn service_not_found(msg: impl Into<String>) -> Self {
        Error::ServiceNotFound(msg.into())
    }

    /// Create a new dependency not found error
    pub fn dependency_not_found(msg: impl Into<String>) -> Self {
        Error::DependencyNotFound(msg.into())
    }

    /// Create a new conflict error
    pub fn conflict(msg: impl Into<String>) -> Self {
        Error::Conflict(msg.into())
    }

    /// Create a new rate limit error
    pub fn rate_limit(msg: impl Into<String>) -> Self {
        Error::RateLimit(msg.into())
    }

    /// Create a new authentication error
    pub fn authentication(msg: impl Into<String>) -> Self {
        Error::Authentication(msg.into())
    }

    /// Create a new authorization error
    pub fn authorization(msg: impl Into<String>) -> Self {
        Error::Authorization(msg.into())
    }

    /// Create a new connection error
    pub fn connection(msg: impl Into<String>) -> Self {
        Error::Connection(msg.into())
    }

    /// Create a new transaction error
    pub fn transaction(msg: impl Into<String>) -> Self {
        Error::Transaction(msg.into())
    }

    /// Create a new query error
    pub fn query(msg: impl Into<String>) -> Self {
        Error::Query(msg.into())
    }

    /// Create a new invalid state error
    pub fn invalid_state(msg: impl Into<String>) -> Self {
        Error::InvalidState(msg.into())
    }

    /// Create a new unsupported operation error
    pub fn unsupported(msg: impl Into<String>) -> Self {
        Error::Unsupported(msg.into())
    }

    /// Create a new resource not found error
    pub fn resource_not_found(msg: impl Into<String>) -> Self {
        Error::ResourceNotFound(msg.into())
    }

    /// Create a new resource already exists error
    pub fn resource_exists(msg: impl Into<String>) -> Self {
        Error::ResourceExists(msg.into())
    }

    /// Create a new internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Error::Internal(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Graph(_)
                | Self::Discovery(_)
                | Self::Storage(_)
                | Self::Network(_)
                | Self::Kubernetes(_)
                | Self::GraphDatabase(_)
                | Self::EventStore(_)
                | Self::Connection(_)
                | Self::Http(_)
                | Self::Timeout(_)
                | Self::RateLimit(_)
        )
    }

    /// Check if error is a fatal error (not retryable)
    pub fn is_fatal(&self) -> bool {
        matches!(
            self,
            Self::Validation(_)
                | Self::Config(_)
                | Self::Serialization(_)
                | Self::ServiceNotFound(_)
                | Self::DependencyNotFound(_)
                | Self::Conflict(_)
                | Self::Authentication(_)
                | Self::Authorization(_)
                | Self::InvalidState(_)
                | Self::Unsupported(_)
                | Self::ResourceNotFound(_)
                | Self::ResourceExists(_)
        )
    }
}

impl From<reqwest::Error> for Error {
    fn from(err: reqwest::Error) -> Self {
        Error::Network(err.to_string())
    }
}

impl From<kube::Error> for Error {
    fn from(err: kube::Error) -> Self {
        Error::Kubernetes(err.to_string())
    }
}

#[cfg(feature = "neo4j")]
impl From<neo4rs::Error> for Error {
    fn from(err: neo4rs::Error) -> Self {
        Error::GraphDatabase(err.to_string())
    }
}

impl From<chrono::ParseError> for Error {
    fn from(err: chrono::ParseError) -> Self {
        Error::Validation(err.to_string())
    }
}

impl From<uuid::Error> for Error {
    fn from(err: uuid::Error) -> Self {
        Error::Validation(err.to_string())
    }
}

impl From<CommonError> for Error {
    fn from(err: CommonError) -> Self {
        match err {
            CommonError::Config { message } => Error::Config(message),
            CommonError::Network { message } => Error::Network(message),
            CommonError::Auth => Error::Authentication("Authentication failed".to_string()),
            CommonError::Authorization { reason } => Error::Authorization(reason),
            CommonError::NotFound {
                resource,
                identifier,
            } => Error::ResourceNotFound(format!("{} '{}'", resource, identifier)),
            CommonError::InvalidInput { message } => Error::Validation(message),
            _ => Error::Internal(err.to_string()),
        }
    }
}

/// Error extension for context
pub trait ErrorContext<T> {
    /// Add context to error
    fn context(self, msg: impl Into<String>) -> Result<T>;
}

impl<T, E: Into<Error>> ErrorContext<T> for std::result::Result<T, E> {
    fn context(self, msg: impl Into<String>) -> Result<T> {
        self.map_err(|e| {
            let mut error = e.into();
            // Add context to the error
            match &mut error {
                Error::Graph(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Discovery(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Storage(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Validation(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Config(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Network(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Kubernetes(s) => *s = format!("{}: {}", msg.into(), s),
                Error::GraphDatabase(s) => *s = format!("{}: {}", msg.into(), s),
                Error::EventStore(s) => *s = format!("{}: {}", msg.into(), s),
                Error::ImpactAnalysis(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Serialization(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Http(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Timeout(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Permission(s) => *s = format!("{}: {}", msg.into(), s),
                Error::ServiceNotFound(s) => *s = format!("{}: {}", msg.into(), s),
                Error::DependencyNotFound(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Conflict(s) => *s = format!("{}: {}", msg.into(), s),
                Error::RateLimit(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Authentication(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Authorization(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Connection(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Transaction(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Query(s) => *s = format!("{}: {}", msg.into(), s),
                Error::InvalidState(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Unsupported(s) => *s = format!("{}: {}", msg.into(), s),
                Error::ResourceNotFound(s) => *s = format!("{}: {}", msg.into(), s),
                Error::ResourceExists(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Internal(s) => *s = format!("{}: {}", msg.into(), s),
                Error::Io(_) | Error::Json(_) | Error::Anyhow(_) => {
                    // These have their own error sources
                }
            }
            error
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let error = Error::graph("Test graph error");
        assert!(matches!(error, Error::Graph(_)));
        assert_eq!(
            error.to_string(),
            "Graph operation failed: Test graph error"
        );
    }

    #[test]
    fn test_error_retryable() {
        let retryable_err = Error::connection("timeout".to_string());
        assert!(retryable_err.is_retryable());

        let fatal_err = Error::validation("invalid input".to_string());
        assert!(!fatal_err.is_retryable());
        assert!(fatal_err.is_fatal());
    }

    #[test]
    fn test_error_conversion() {
        let io_error = std::io::Error::new(std::io::ErrorKind::NotFound, "File not found");
        let error: Error = io_error.into();
        assert!(matches!(error, Error::Io(_)));
    }

    #[test]
    fn test_error_context() {
        let result: Result<()> = Err(Error::graph("Original error"));
        let result = result.context("Additional context");

        assert!(result.is_err());
        let error = result.unwrap_err();
        assert!(error.to_string().contains("Additional context"));
        assert!(error.to_string().contains("Original error"));
    }

    #[test]
    fn test_error_kinds() {
        let errors = vec![
            Error::graph("graph"),
            Error::discovery("discovery"),
            Error::storage("storage"),
            Error::validation("validation"),
            Error::config("config"),
            Error::network("network"),
            Error::kubernetes("kubernetes"),
            Error::graph_database("database"),
            Error::event_store("event"),
            Error::impact_analysis("impact"),
            Error::serialization("serialization"),
            Error::http("http"),
            Error::timeout("timeout"),
            Error::permission("permission"),
            Error::service_not_found("service"),
            Error::dependency_not_found("dependency"),
            Error::conflict("conflict"),
            Error::rate_limit("rate"),
            Error::authentication("auth"),
            Error::authorization("authz"),
            Error::connection("connection"),
            Error::transaction("tx"),
            Error::query("query"),
            Error::invalid_state("state"),
            Error::unsupported("unsupported"),
            Error::resource_not_found("resource"),
            Error::resource_exists("exists"),
            Error::internal("internal"),
        ];

        for error in errors {
            assert!(!error.to_string().is_empty());
        }
    }
}
