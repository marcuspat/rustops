//! # RustOps Incident Management
//!
//! Bounded context for incident management following ADR-0007.
//!
//! ## Architecture
//!
//! This crate implements:
//! - Alert correlation and deduplication
//! - Topological grouping
//! - Root cause ranking
//! - CQRS read/write models
//! - Event sourcing for incidents
//!
//! ## Key Components
//!
//! - **Correlation**: Groups related alerts into incidents
//! - **Deduplication**: Removes duplicate alerts (5-minute window)
//! - **Repository**: Persistence layer with CQRS
//! - **Service Graph**: Topological analysis

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod correlation;
pub mod deduplication;
pub mod events;
pub mod incident;
pub mod repository;

pub use correlation::{AlertCorrelator, AlertGroup, CorrelationConfig};
pub use deduplication::{AlertDeduplicator, Fingerprinter};
pub use events::{IncidentEvent, IncidentEventStore};
pub use incident::{Incident, IncidentRepository, IncidentStatus, Severity};
pub use repository::{CQRSProjection, ReadModel, WriteModel};

use rustops_common::{Error, Result};
use std::collections::HashMap;
use std::time::Duration;
