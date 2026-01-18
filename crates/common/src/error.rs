//! Common error types for RustOps.

use thiserror::Error;

/// Result type for RustOps operations.
pub type Result<T> = std::result::Result<T, Error>;

/// Common error type.
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error.
    #[error("configuration error: {0}")]
    Config(String),

    /// Network error.
    #[error("network error: {0}")]
    Network(String),

    /// Authentication error.
    #[error("authentication failed")]
    Auth,

    /// Not found error.
    #[error("not found: {0}")]
    NotFound(String),

    /// Invalid input.
    #[error("invalid input: {0}")]
    InvalidInput(String),

    /// Internal error.
    #[error("internal error: {0}")]
    Internal(String),
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::Config("missing field".to_string());
        assert_eq!(err.to_string(), "configuration error: missing field");
    }
}
