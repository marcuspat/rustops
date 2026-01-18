//! Mock services for testing.

pub mod prometheus;
pub mod kafka;
pub mod http;

pub use prometheus::MockPrometheus;
pub use kafka::MockKafka;
pub use http::MockHttpServer;
