//! Configuration types for RustOps.

use serde::{Deserialize, Serialize};

/// Global configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Agent configuration.
    pub agent: AgentConfig,
    /// Pipeline configuration.
    pub pipeline: PipelineConfig,
}

/// Agent configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConfig {
    /// Collection interval in seconds.
    pub collection_interval_seconds: u64,
}

/// Pipeline configuration.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PipelineConfig {
    /// Kafka brokers.
    pub kafka_brokers: Vec<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            agent: AgentConfig {
                collection_interval_seconds: 15,
            },
            pipeline: PipelineConfig {
                kafka_brokers: vec!["localhost:9092".to_string()],
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.agent.collection_interval_seconds, 15);
    }
}
