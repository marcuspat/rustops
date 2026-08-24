//! RustOps - AIOps platform for automated monitoring, anomaly detection, and incident remediation.
//!
//! Workspace root crate. See https://github.com/marcuspat/rustops for the full platform.

pub fn version() -> &'static str { env!("CARGO_PKG_VERSION") }
