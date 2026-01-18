//! Testing utilities and builders for RustOps.
//!
//! This module provides test data builders, fixtures, and utilities
//! for writing tests across the RustOps workspace.

pub mod builders;
pub mod data;

pub use builders::{
    MetricBuilder,
    LogEntryBuilder,
    ConfigBuilder,
};
pub use data::{
    TestDataGenerator,
    FixtureLoader,
};

/// Initialize tracing for tests.
pub fn init_test_tracing() {
    let _ = tracing_subscriber::fmt()
        .with_test_writer()
        .try_init();
}

/// Creates a temporary directory for test files.
pub fn temp_dir() -> tempfile::TempDir {
    tempfile::tempdir().expect("Failed to create temp dir")
}

/// Asserts that two floats are approximately equal.
pub fn assert_approx_eq(a: f64, b: f64, epsilon: f64) {
    let diff = (a - b).abs();
    assert!(
        diff <= epsilon,
        "Values not approximately equal: {} vs {} (diff: {})",
        a, b, diff
    );
}
