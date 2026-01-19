//! # Error types for RustOps
//!
//! Comprehensive error handling using thiserror for domain-specific errors
//! and anyhow for application-level errors.

use std::path::PathBuf;
use thiserror::Error;

/// Result type for RustOps operations
pub type Result<T> = std::result::Result<T, Error>;

/// Common error type for the RustOps platform
#[derive(Error, Debug)]
pub enum Error {
    /// Configuration error
    #[error("configuration error: {message}")]
    Config {
        /// Human-readable error message describing the configuration issue
        message: String,
    },

    /// Network error
    #[error("network error: {message}")]
    Network {
        /// Human-readable error message describing the network issue
        message: String,
    },

    /// Authentication error
    #[error("authentication failed")]
    Auth,

    /// Authorization error
    #[error("authorization denied: {reason}")]
    Authorization {
        /// Human-readable explanation for why authorization was denied
        reason: String,
    },

    /// Not found error
    #[error("not found: {resource} '{identifier}'")]
    NotFound {
        /// Type of resource that was not found (e.g., "service", "incident")
        resource: String,
        /// Identifier of the specific resource that was not found
        identifier: String,
    },

    /// Invalid input
    #[error("invalid input: {message}")]
    InvalidInput {
        /// Human-readable error message describing the validation issue
        message: String,
    },

    /// Internal error
    #[error("internal error: {message}")]
    Internal {
        /// Human-readable error message describing the internal issue
        message: String,
    },

    /// Database error
    #[error("database error: {message}")]
    Database {
        /// Human-readable error message describing the database issue
        message: String,
    },

    /// Serialization error
    #[error("serialization error: {message}")]
    Serialization {
        /// Human-readable error message describing the serialization failure
        message: String,
    },

    /// Deserialization error
    #[error("deserialization error: {message}")]
    Deserialization {
        /// Human-readable error message describing the deserialization failure
        message: String,
    },

    /// Validation error
    #[error("validation error: {message}")]
    Validation {
        /// Human-readable error message describing the validation failure
        message: String,
    },

    /// Timeout error
    #[error("operation timed out after {duration_ms}ms")]
    Timeout {
        /// Duration in milliseconds before the operation timed out
        duration_ms: u64,
    },

    /// Rate limit exceeded
    #[error("rate limit exceeded: {limit} requests per {window_secs}s")]
    RateLimit {
        /// Maximum number of requests allowed within the time window
        limit: u32,
        /// Time window in seconds
        window_secs: u32,
    },

    /// Service unavailable
    #[error("service unavailable: {service}")]
    ServiceUnavailable {
        /// Name of the service that is unavailable
        service: String,
    },

    /// Telemetry ingestion error
    #[error("telemetry ingestion error: {message}")]
    TelemetryIngestion {
        /// Human-readable error message describing the ingestion issue
        message: String,
    },

    /// Anomaly detection error
    #[error("anomaly detection error: {message}")]
    AnomalyDetection {
        /// Human-readable error message describing the detection issue
        message: String,
    },

    /// Model loading error
    #[error("model loading error: {model_name}: {message}")]
    ModelLoading {
        /// Name of the model that failed to load
        model_name: String,
        /// Human-readable error message describing the loading failure
        message: String,
    },

    /// Model inference error
    #[error("model inference error: {model_name}: {message}")]
    ModelInference {
        /// Name of the model that failed during inference
        model_name: String,
        /// Human-readable error message describing the inference failure
        message: String,
    },

    /// Kafka/Streaming error
    #[error("kafka error: {message}")]
    Kafka {
        /// Human-readable error message describing the Kafka issue
        message: String,
    },

    /// Correlation error
    #[error("correlation error: {message}")]
    Correlation {
        /// Human-readable error message describing the correlation issue
        message: String,
    },

    /// Incident error
    #[error("incident error: {message}")]
    Incident {
        /// Human-readable error message describing the incident issue
        message: String,
    },

    /// File I/O error
    #[error("file I/O error: {path}: {message}")]
    Io {
        /// Path to the file that caused the I/O error
        path: PathBuf,
        /// Human-readable error message describing the I/O issue
        message: String,
    },

    /// Parse error
    #[error("parse error: {message}")]
    Parse {
        /// Human-readable error message describing the parsing failure
        message: String,
    },
}

impl Error {
    /// Create a configuration error
    pub fn config(message: impl Into<String>) -> Self {
        Self::Config {
            message: message.into(),
        }
    }

    /// Create a network error
    pub fn network(message: impl Into<String>) -> Self {
        Self::Network {
            message: message.into(),
        }
    }

    /// Create a not found error
    pub fn not_found(resource: impl Into<String>, identifier: impl Into<String>) -> Self {
        Self::NotFound {
            resource: resource.into(),
            identifier: identifier.into(),
        }
    }

    /// Create an invalid input error
    pub fn invalid_input(message: impl Into<String>) -> Self {
        Self::InvalidInput {
            message: message.into(),
        }
    }

    /// Create an internal error
    pub fn internal(message: impl Into<String>) -> Self {
        Self::Internal {
            message: message.into(),
        }
    }

    /// Create a validation error
    pub fn validation(message: impl Into<String>) -> Self {
        Self::Validation {
            message: message.into(),
        }
    }

    /// Create a timeout error
    pub fn timeout(duration_ms: u64) -> Self {
        Self::Timeout { duration_ms }
    }

    /// Check if error is retryable
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::Network { .. }
                | Self::Timeout { .. }
                | Self::ServiceUnavailable { .. }
                | Self::RateLimit { .. }
                | Self::Database { .. }
        )
    }

    /// Check if error is transient (might resolve itself)
    pub fn is_transient(&self) -> bool {
        matches!(
            self,
            Self::Timeout { .. } | Self::RateLimit { .. } | Self::ServiceUnavailable { .. }
        )
    }
}

// Conversion from common error types
impl From<std::io::Error> for Error {
    fn from(err: std::io::Error) -> Self {
        Self::Io {
            path: PathBuf::from("unknown"),
            message: err.to_string(),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(err: serde_json::Error) -> Self {
        if err.is_data() {
            Self::Deserialization {
                message: err.to_string(),
            }
        } else {
            Self::Serialization {
                message: err.to_string(),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_display() {
        let err = Error::config("missing field");
        assert!(err.to_string().contains("configuration error"));
    }

    #[test]
    fn test_not_found_error() {
        let err = Error::not_found("service", "my-service");
        assert!(err.to_string().contains("service"));
        assert!(err.to_string().contains("my-service"));
    }

    #[test]
    fn test_retryable_errors() {
        assert!(Error::timeout(1000).is_retryable());
        assert!(Error::network("connection refused".to_string()).is_retryable());
        assert!(!Error::invalid_input("bad value").is_retryable());
    }

    #[test]
    fn test_transient_errors() {
        assert!(Error::timeout(1000).is_transient());
        assert!(!Error::network("connection refused".to_string()).is_transient());
    }
}
