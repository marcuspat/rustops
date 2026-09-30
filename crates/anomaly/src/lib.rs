//! # RustOps Anomaly Detection
//!
//! Bounded context for detecting anomalies in telemetry data.
//!
//! ## What is implemented
//!
//! - **Statistical detection** — Z-score (leave-one-out baseline) and IQR
//!   detectors in [`statistical`]
//! - **Routing** — [`router`] dispatches metrics to the registered detectors
//!
//! ## What is NOT implemented
//!
//! - **ONNX/ML inference** — [`models`] is an explicit stub: loading a model
//!   returns an error until the `ort` dependency is enabled and the code is
//!   finished. There is no ML detection in this crate today.
//! - **CUSUM / seasonal / clustering** — not written yet.

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
