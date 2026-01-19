//! # RustOps Telemetry Agent
//!
//! A telemetry collection agent that:
//! - Collects metrics from Prometheus
//! - Submits to a pipeline (Kafka)
//! - Runs in a loop with configurable interval
//! - Has graceful shutdown

#![warn(missing_docs)]
#![warn(clippy::all)]

use chrono::Utc;
use rustops_common::ServiceId;
use rustops_integration::{
    CircuitBreakerConfig, IntegrationAdapter, PrometheusAdapter, RateLimiterConfig, RetryConfig,
};
use rustops_telemetry::{KafkaProducer, MetricsCollector};
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

/// Agent configuration
#[derive(Debug, Clone)]
struct AgentConfig {
    /// Prometheus base URL
    pub prometheus_url: String,

    /// Collection interval in seconds
    pub interval_secs: u64,

    /// Service ID for the agent
    pub service_id: ServiceId,

    /// Queries to execute
    pub queries: Vec<String>,
}

impl Default for AgentConfig {
    fn default() -> Self {
        Self {
            prometheus_url: "http://localhost:9090".to_string(),
            interval_secs: 15,
            service_id: ServiceId::new(),
            queries: vec![
                "up".to_string(),
                "process_cpu_seconds_total".to_string(),
                "go_goroutines".to_string(),
            ],
        }
    }
}

/// Telemetry collection agent
struct Agent {
    config: AgentConfig,
    prometheus: PrometheusAdapter,
    collector: MetricsCollector,
    producer: Arc<KafkaProducer>,
}

impl Agent {
    /// Create a new agent
    fn new(config: AgentConfig) -> Result<Self, rustops_common::Error> {
        // Create Prometheus adapter
        let prometheus = PrometheusAdapter::new(
            "rustops-agent-prometheus",
            &config.prometheus_url,
            None::<(&str, &str)>,
            CircuitBreakerConfig::default(),
            RateLimiterConfig::default(),
            RetryConfig::default(),
        );

        // Create Kafka producer (stub for now)
        let producer = Arc::new(
            KafkaProducer::new(config.service_id)
                .map_err(|e| rustops_common::Error::internal(format!("Failed to create producer: {}", e)))?,
        );

        // Create metrics collector
        let collector = MetricsCollector::new(producer.clone(), config.service_id);

        Ok(Self {
            config,
            prometheus,
            collector,
            producer,
        })
    }

    /// Initialize the agent
    async fn initialize(&mut self) -> Result<(), rustops_common::Error> {
        info!("Initializing RustOps telemetry agent");
        info!("Prometheus URL: {}", self.config.prometheus_url);
        info!("Collection interval: {}s", self.config.interval_secs);
        info!("Service ID: {}", self.config.service_id);

        // Initialize Prometheus adapter
        self.prometheus.initialize().await.map_err(|e| {
            rustops_common::Error::internal(format!("Failed to initialize Prometheus: {}", e))
        })?;

        info!("Agent initialized successfully");
        Ok(())
    }

    /// Run a single collection cycle
    async fn collect_cycle(&self) -> Result<(), rustops_common::Error> {
        info!("Starting collection cycle at {}", Utc::now());

        for query in &self.config.queries {
            info!("Executing query: {}", query);

            match self.prometheus.query_instant(query).await {
                Ok(response) => {
                    info!("Query '{}' returned {} results", query, response.data.result.len());

                    // Process each metric result
                    for result in response.data.result {
                        // Convert to Prometheus text format line
                        let metric_name = query.clone();
                        let labels: Vec<String> = result
                            .metric
                            .iter()
                            .map(|(k, v)| format!(r#"{}="{}""#, k, v))
                            .collect();

                        let line = if labels.is_empty() {
                            format!("{} {}", metric_name, "1")
                        } else {
                            format!(r#"{{{}}}{} {}"#, labels.join(","), metric_name, "1")
                        };

                        // Collect the metric line
                        if let Err(e) = self.collector.collect_line(&line).await {
                            error!("Failed to collect metric line: {}", e);
                        }
                    }
                }
                Err(e) => {
                    warn!("Query '{}' failed: {}", query, e);
                }
            }
        }

        info!("Collection cycle completed at {}", Utc::now());
        Ok(())
    }

    /// Run the agent's main loop
    async fn run(&self) -> Result<(), rustops_common::Error> {
        info!("Starting agent main loop");

        let mut interval = tokio::time::interval(Duration::from_secs(self.config.interval_secs));

        loop {
            interval.tick().await;

            if let Err(e) = self.collect_cycle().await {
                error!("Collection cycle failed: {}", e);
            }
        }
    }

    /// Shutdown the agent
    async fn shutdown(&mut self) -> Result<(), rustops_common::Error> {
        info!("Shutting down agent");

        self.prometheus.shutdown().await.map_err(|e| {
            rustops_common::Error::internal(format!("Failed to shutdown Prometheus: {}", e))
        })?;

        info!("Agent shutdown complete");
        Ok(())
    }
}

/// Setup graceful shutdown signal handling
async fn wait_for_shutdown() {
    let ctrl_c = async {
        signal::ctrl_c()
            .await
            .expect("failed to install Ctrl+C handler");
    };

    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler")
            .recv()
            .await;
    };

    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = ctrl_c => {
            info!("Received Ctrl+C signal");
        }
        _ = terminate => {
            info!("Received terminate signal");
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rustops_agent=debug,info".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    info!("RustOps Telemetry Agent starting...");

    // Load configuration (use defaults for now)
    let config = AgentConfig::default();

    // Create and initialize agent
    let mut agent = Agent::new(config)?;
    agent.initialize().await?;

    // Spawn the agent run loop in a task
    let agent_task = tokio::spawn(async move {
        if let Err(e) = agent.run().await {
            error!("Agent run loop failed: {}", e);
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown().await;

    info!("Shutdown signal received, stopping agent...");

    // Cancel the agent task
    agent_task.abort();

    info!("Agent stopped");

    Ok(())
}
