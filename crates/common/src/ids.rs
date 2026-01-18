//! # Type-safe IDs using newtype pattern
//!
//! Provides compile-time type safety for entity IDs throughout the system.
//! Prevents mixing up different ID types (e.g., using an IncidentId where a ServiceId is expected).

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// Marker trait for all ID types
pub trait IdType: Clone + Copy + PartialEq + Eq + PartialOrd + Ord + Send + Sync + 'static {
    /// Create a new random ID
    fn new() -> Self;

    /// Get the underlying UUID
    fn as_uuid(&self) -> Uuid;

    /// Create from a UUID string
    fn from_str(s: &str) -> Result<Self, uuid::Error>
    where
        Self: Sized;

    /// Convert to string
    fn to_string(&self) -> String;
}

/// Macro to implement newtype ID wrapper
macro_rules! impl_id {
    ($name:ident, $prefix:expr) => {
        #[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
        #[serde(transparent)]
        pub struct $name(Uuid);

        impl $name {
            /// Create a new random ID
            pub fn new() -> Self {
                Self(Uuid::new_v4())
            }

            /// Get the underlying UUID
            pub fn as_uuid(&self) -> Uuid {
                self.0
            }

            /// Create from a UUID
            pub fn from_uuid(uuid: Uuid) -> Self {
                Self(uuid)
            }

            /// Convert to string with prefix
            pub fn to_prefixed_string(&self) -> String {
                format!("{}{}", $prefix, self.0)
            }
        }

        impl IdType for $name {
            fn new() -> Self {
                Self::new()
            }

            fn as_uuid(&self) -> Uuid {
                self.0
            }

            fn from_str(s: &str) -> Result<Self, uuid::Error> {
                Ok(Self(Uuid::parse_str(s)?))
            }

            fn to_string(&self) -> String {
                self.0.to_string()
            }
        }

        impl Default for $name {
            fn default() -> Self {
                Self::new()
            }
        }

        impl std::fmt::Display for $name {
            fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                write!(f, "{}{}", $prefix, self.0)
            }
        }

        impl std::str::FromStr for $name {
            type Err = uuid::Error;

            fn from_str(s: &str) -> Result<Self, Self::Err> {
                // Strip prefix if present
                let s = s.strip_prefix($prefix).unwrap_or(s);
                Ok(Self(Uuid::parse_str(s)?))
            }
        }
    };
}

// Domain-specific ID types
impl_id!(IncidentId, "inc_");
impl_id!(AlertId, "alt_");
impl_id!(ServiceId, "svc_");
impl_id!(MetricId, "mtr_");
impl_id!(TraceId, "trc_");
impl_id!(SpanId, "spn_");
impl_id!(AnomalyId, "anm_");
impl_id!(CorrelationId, "cor_");
impl_id!(UserId, "usr_");
impl_id!(ResourceId, "res_");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_id_creation() {
        let id = IncidentId::new();
        assert_ne!(id.as_uuid(), Uuid::nil());
    }

    #[test]
    fn test_id_from_str() {
        let uuid_str = "550e8400-e29b-41d4-a716-446655440000";
        let id = IncidentId::from_str(uuid_str).unwrap();
        assert_eq!(id.to_string(), uuid_str);
    }

    #[test]
    fn test_id_type_safety() {
        let incident_id = IncidentId::new();
        let alert_id = AlertId::new();
        // This should not compile - type safety enforced at compile time
        // assert_eq!(incident_id, alert_id);
    }

    #[test]
    fn test_id_serialization() {
        let id = IncidentId::new();
        let json = serde_json::to_string(&id).unwrap();
        let deserialized: IncidentId = serde_json::from_str(&json).unwrap();
        assert_eq!(id, deserialized);
    }

    #[test]
    fn test_id_display() {
        let id = IncidentId::new();
        let s = id.to_string();
        assert!(s.starts_with("inc_"));
    }
}
