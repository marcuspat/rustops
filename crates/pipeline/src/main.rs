//! RustOps Event Processing Pipeline
//!
//! This service pushes telemetry through normalization, anomaly detection,
//! and topology updates. There is NO Kafka consumer yet: a synthetic
//! in-process source emits Prometheus text-format samples so the real
//! pipeline path is exercised end to end. Kafka ingestion is future work.

use rustops_anomaly::{Anomaly, AnomalyDetector, ZScoreDetector};
use rustops_common::{Metric, Result, ServiceId};
use rustops_telemetry::{
    TelemetryEnvelope, TelemetryFormat, TelemetryNormalizer, TelemetryPayload, TelemetryType,
};
use rustops_topology::{ServiceGraph, ServiceNode, ServiceType};
use std::collections::VecDeque;
use std::sync::Arc;
use std::time::Duration;
use tokio::signal;
use tracing::{info, warn};

/// Sliding-window size used as the anomaly-detection baseline.
const DETECTION_WINDOW: usize = 64;

/// Pipeline configuration
#[derive(Debug, Clone)]
struct PipelineConfig {
    kafka_brokers: String,
    consumer_group: String,
    poll_interval_ms: u64,
}

impl Default for PipelineConfig {
    fn default() -> Self {
        Self {
            kafka_brokers: std::env::var("KAFKA_BROKERS")
                .unwrap_or_else(|_| "localhost:9092".to_string()),
            consumer_group: "rustops-pipeline".to_string(),
            poll_interval_ms: 1000,
        }
    }
}

/// Event processing pipeline
struct Pipeline {
    config: PipelineConfig,
    normalizer: Arc<TelemetryNormalizer>,
    z_score_detector: ZScoreDetector,
    service_graph: Arc<tokio::sync::RwLock<ServiceGraph>>,
    /// Sliding window of recent metrics used as the detection baseline.
    recent: VecDeque<Metric>,
}

impl Pipeline {
    fn new(config: PipelineConfig) -> Self {
        let service_id = ServiceId::new();
        Self {
            config,
            normalizer: Arc::new(TelemetryNormalizer::new(service_id)),
            z_score_detector: ZScoreDetector::new(2.0),
            service_graph: Arc::new(tokio::sync::RwLock::new(ServiceGraph::new(None))),
            recent: VecDeque::with_capacity(DETECTION_WINDOW),
        }
    }

    /// Process a single telemetry envelope
    async fn process_envelope(&mut self, envelope: TelemetryEnvelope) -> Result<()> {
        info!(
            "Processing telemetry of type: {:?}",
            envelope.telemetry_type
        );

        // Extract metric if available
        if let TelemetryPayload::Metric(metric) = envelope.payload {
            // Detect anomalies
            let anomalies = self.detect_anomalies(&metric).await?;
            if let Some(anomaly) = anomalies.first() {
                info!(
                    "Anomaly detected: score={}, type={:?}",
                    anomaly.score, anomaly.anomaly_type
                );

                // Could trigger incident creation here
                self.handle_anomaly(anomaly.clone()).await?;
            }

            // Update topology with service information
            self.update_topology_for_metric(&metric).await?;
        }

        Ok(())
    }

    /// Detect anomalies for the newly ingested metric against the window
    /// of recent samples (the detector needs a baseline to score against).
    async fn detect_anomalies(&mut self, metric: &Metric) -> Result<Vec<Anomaly>> {
        self.recent.push_back(metric.clone());
        if self.recent.len() > DETECTION_WINDOW {
            self.recent.pop_front();
        }

        let window: Vec<Metric> = self.recent.iter().cloned().collect();
        let result = self.z_score_detector.detect(&window).await?;

        // Only surface anomalies for the metric just ingested; older window
        // entries were already reported on their own iteration.
        Ok(result
            .anomalies
            .into_iter()
            .filter(|a| a.metric_id == metric.id)
            .collect())
    }

    /// Handle detected anomaly
    async fn handle_anomaly(&mut self, anomaly: Anomaly) -> Result<()> {
        // TODO: Create incident, send alert, etc.
        warn!("Anomaly detected but not yet handled: {:?}", anomaly.id);
        Ok(())
    }

    /// Update service topology based on metric
    async fn update_topology_for_metric(&self, metric: &Metric) -> Result<()> {
        let service_id = metric.service_id;
        let mut graph = self.service_graph.write().await;

        // Check if service exists
        if graph.get_service(&service_id).is_none() {
            // Add service to topology
            let service_name = metric
                .labels
                .get("service_name")
                .cloned()
                .unwrap_or_else(|| format!("service-{}", service_id));

            let namespace = metric
                .labels
                .get("namespace")
                .cloned()
                .unwrap_or_else(|| "default".to_string());

            let cluster = metric
                .labels
                .get("cluster")
                .cloned()
                .unwrap_or_else(|| "default-cluster".to_string());

            let service = ServiceNode::new(
                service_id,
                Some(service_name),
                namespace,
                cluster,
                ServiceType::Deployment,
            );

            graph.add_service(service).map_err(|e| {
                rustops_common::Error::internal(format!("Failed to add service: {}", e))
            })?;

            info!("Added service to topology: {}", service_id);
        }

        Ok(())
    }

    /// Run the main processing loop.
    ///
    /// No Kafka consumer exists yet, so a synthetic in-process source emits
    /// Prometheus text-format samples — a steady baseline with a periodic
    /// spike — and pushes each one through the real normalize → detect →
    /// topology path.
    async fn run(&mut self) -> Result<()> {
        info!("Starting pipeline loop (synthetic source; Kafka ingestion not implemented)");

        let interval = Duration::from_millis(self.config.poll_interval_ms);
        let mut iterations = 0u64;
        loop {
            tokio::select! {
                _ = tokio::time::sleep(interval) => {
                    iterations += 1;
                    // Baseline ~50 with jitter; every 32nd sample spikes.
                    let value = if iterations.is_multiple_of(32) {
                        250.0
                    } else {
                        50.0 + (iterations % 8) as f64
                    };
                    let raw = format!(
                        "cpu_usage_percent{{service_name=\"synthetic-demo\",namespace=\"default\"}} {value}"
                    );

                    match self.normalizer.normalize_metric(&raw, TelemetryFormat::Prometheus) {
                        Ok(metric) => {
                            let envelope = TelemetryEnvelope {
                                telemetry_type: TelemetryType::Metric,
                                timestamp: chrono::Utc::now(),
                                payload: TelemetryPayload::Metric(metric),
                                metadata: std::collections::HashMap::new(),
                            };
                            if let Err(e) = self.process_envelope(envelope).await {
                                warn!("Failed to process envelope: {e}");
                            }
                        }
                        Err(e) => warn!("Failed to normalize synthetic sample: {e}"),
                    }
                }
                _ = shutdown_signal() => {
                    info!("Shutdown signal received");
                    return Ok(());
                }
            }
        }
    }
}

/// Wait for shutdown signal (SIGINT or SIGTERM)
async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let ctrl_c = signal::ctrl_c();
        let mut terminate = signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("failed to install signal handler");
        tokio::select! {
            _ = ctrl_c => {}
            _ = terminate.recv() => {}
        }
    }

    #[cfg(not(unix))]
    {
        let _ = signal::ctrl_c().await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize tracing
    tracing_subscriber::fmt()
        .with_env_filter(
            std::env::var("RUST_LOG").unwrap_or_else(|_| "info,rustops_pipeline=debug".to_string()),
        )
        .init();

    info!("RustOps Pipeline starting...");

    // Load configuration
    let config = PipelineConfig::default();

    info!("Configuration loaded:");
    info!(
        "  Kafka brokers (configured, consumer not yet implemented): {}",
        config.kafka_brokers
    );
    info!("  Consumer group: {}", config.consumer_group);
    info!("  Poll interval: {}ms", config.poll_interval_ms);

    // Create pipeline
    let mut pipeline = Pipeline::new(config);

    // Run pipeline
    pipeline.run().await?;

    info!("RustOps Pipeline shut down gracefully");
    Ok(())
}
