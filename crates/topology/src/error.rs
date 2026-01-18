//! Error types for the topology service

use thiserror::Error;

/// Result type for topology operations
pub type Result<T> = std::result::Result<T, Error>;

/// Errors that can occur in the topology service
#[derive(Error, Debug)]
pub enum Error {
    /// Graph database error
    #[error("Graph database error: {0}")]
    GraphDatabase(String),

    /// Discovery error
    #[error("Discovery error: {0}")]
    Discovery(String),

    /// Query error
    #[error("Query error: {0}")]
    Query(String),

    /// Connection error
    #[error("Connection error: {0}")]
    Connection(String),

    /// Serialization error
    #[error("Serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Kubernetes API error
    #[cfg(feature = "kubernetes")]
    #[error("Kubernetes error: {0}")]
    Kubernetes(String),

    /// HTTP client error
    #[error("HTTP error: {0}")]
    Http(String),

    /// Parse error
    #[error("Parse error: {0}")]
    Parse(String),

    /// Validation error
    #[error("Validation error: {0}")]
    Validation(String),

    /// Generic internal error
    #[error("Internal error: {0}")]
    Internal(String),
}

impl Error {
    /// Create a graph database error
    pub fn graph_database(msg: impl Into<String>) -> Self {
        Self::GraphDatabase(msg.into())
    }

    /// Create a discovery error
    pub fn discovery(msg: impl Into<String>) -> Self {
        Self::Discovery(msg.into())
    }

    /// Create a query error
    pub fn query(msg: impl Into<String>) -> Self {
        Self::Query(msg.into())
    }

    /// Create an internal error
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal(msg.into())
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Connection(_)
                | Self::Http(_)
                | #[cfg(feature = "kubernetes")]
                Self::Kubernetes(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_retryable() {
        let err = Error::Connection("timeout".to_string());
        assert!(err.is_retryable());

        let err = Error::Validation("invalid input".to_string());
        assert!(!err.is_retryable());
    }
}
