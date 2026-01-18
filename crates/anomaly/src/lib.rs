//! # RustOps Anomaly Detection
//!
//! Bounded context for detecting anomalies in telemetry data.
//!
//! ## Architecture
//!
//! This crate implements the anomaly detection engine from ADR-0006:
//! - Statistical baseline detection (Z-score, IQR, CUSUM)
//! - ONNX ML model integration
//! - Pattern recognition (clustering)
//!
//! ## Key Components
//!
//! - **Detectors**: Various anomaly detection algorithms
//! - **Models**: ONNX model management
//! - **Router**: Routes telemetry to optimal detector
//!
//! ## Design
//!
//! Hybrid approach:
//! - **Statistical**: Fast, rule-based detection (<1ms)
//! - **ML**: Accurate but slower detection (~50ms)
//! - **Pattern**: Known signature matching

#![warn(missing_docs)]
#![warn(clippy::all)]

pub mod detector;
pub mod models;
pub mod router;
pub mod statistical;

pub use detector::{Anomaly, AnomalyDetector, AnomalyType, DetectionResult};
pub use models::{ONNXModel, ONNXModelManager};
pub use router::DetectionRouter;
pub use statistical::{IQRDetector, ZScoreDetector};

use rustops_common::{Error, Metric, MetricId, Result, ServiceId};
use std::collections::HashMap;
use std::time::Duration;
