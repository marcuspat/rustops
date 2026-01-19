//! # Telemetry collectors
//!
//! Collectors for ingesting telemetry from various sources.

use crate::{KafkaProducer, TelemetryEnvelope, TelemetryType};
use chrono::Utc;
use rustops_common::telemetry::LogLevel;
use rustops_common::{Error, LogEntry, Metric, MetricType, Result, ServiceId, TraceSpan};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::mpsc;
use tracing::{debug, error, info};

/// Channel buffer size for telemetry ingestion
const CHANNEL_BUFFER: usize = 10000;

/// Metrics collector - ingests Prometheus-style metrics
pub struct MetricsCollector {
    sender: mpsc::Sender<TelemetryEnvelope>,
    service_id: ServiceId,
}

impl MetricsCollector {
    /// Create a new metrics collector
    pub fn new(producer: Arc<KafkaProducer>, service_id: ServiceId) -> Self {
        let (sender, mut receiver) = mpsc::channel(CHANNEL_BUFFER);
        let producer_clone = producer.clone();

        // Spawn background task to process metrics
        tokio::spawn(async move {
            let mut count = 0u64;
            while let Some(envelope) = receiver.recv().await {
                match producer_clone.produce(envelope).await {
                    Ok(_) => {
                        count += 1;
                        if count % 1000 == 0 {
                            debug!("Produced {} metric envelopes", count);
                        }
                    }
                    Err(e) => {
                        error!("Failed to produce metric envelope: {}", e);
                        // TODO: Dead letter queue
                    }
                }
            }
            info!("Metrics collector task ended");
        });

        Self { sender, service_id }
    }

    /// Collect a raw metric line (Prometheus text format)
    ///
    /// # Example
    ///
    /// ```text
    /// http_requests_total{method="post",code="200"} 1027
    /// ```
    pub async fn collect_line(&self, line: &str) -> Result<()> {
        let metric = self.parse_prometheus_line(line)?;

        let envelope = TelemetryEnvelope {
            telemetry_type: TelemetryType::Metric,
            timestamp: Utc::now(),
            payload: metric.clone().into(),
            metadata: HashMap::new(),
        };

        self.sender
            .send(envelope)
            .await
            .map_err(|e| Error::internal(format!("Failed to send metric: {}", e)))?;

        Ok(())
    }

    /// Parse a Prometheus text format line
    fn parse_prometheus_line(&self, line: &str) -> Result<Metric> {
        // Strip comments and empty lines
        let line = line.split('#').next().unwrap_or(line).trim();
        if line.is_empty() {
            return Err(Error::invalid_input("empty metric line"));
        }

        // Parse: metric_name{labels} value
        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(Error::invalid_input("invalid metric format: missing value"));
        }

        let name_part = parts[0];
        let value: f64 = parts[1]
            .parse()
            .map_err(|_| Error::invalid_input("invalid metric value"))?;

        // Extract metric name and labels
        let (name, labels) = if let Some(brace_start) = name_part.find('{') {
            let name = name_part[..brace_start].to_string();
            let labels_str = &name_part[brace_start + 1..name_part.len() - 1];
            let labels = self.parse_labels(labels_str)?;
            (name, labels)
        } else {
            (name_part.to_string(), HashMap::new())
        };

        // Determine metric type from name
        let metric_type = if name.ends_with("_total") || name.ends_with("_count") {
            MetricType::Counter
        } else if name.ends_with("_bucket") || name.ends_with("_sum") {
            MetricType::Histogram
        } else {
            MetricType::Gauge
        };

        Ok(Metric::new(
            name,
            metric_type,
            value,
            self.service_id,
            labels,
        ))
    }

    /// Parse label string like `method="post",code="200"`
    fn parse_labels(&self, labels_str: &str) -> Result<HashMap<String, String>> {
        let mut labels = HashMap::new();

        for pair in labels_str.split(',') {
            let pair = pair.trim();
            if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].trim().to_string();
                let value = &pair[eq_pos + 1..];
                // Remove quotes
                let value = value
                    .trim_matches('"')
                    .replace("\\\"", "\"")
                    .replace("\\\\", "\\");
                labels.insert(key, value);
            }
        }

        Ok(labels)
    }
}

/// Log collector - ingests log entries
pub struct LogCollector {
    sender: mpsc::Sender<TelemetryEnvelope>,
    service_id: ServiceId,
}

impl LogCollector {
    /// Create a new log collector
    pub fn new(producer: Arc<KafkaProducer>, service_id: ServiceId) -> Self {
        let (sender, mut receiver) = mpsc::channel(CHANNEL_BUFFER);
        let producer_clone = producer.clone();

        tokio::spawn(async move {
            while let Some(envelope) = receiver.recv().await {
                if let Err(e) = producer_clone.produce(envelope).await {
                    error!("Failed to produce log envelope: {}", e);
                }
            }
        });

        Self { sender, service_id }
    }

    /// Collect a log entry
    pub async fn collect(
        &self,
        level: LogLevel,
        message: impl Into<String>,
        labels: HashMap<String, String>,
    ) -> Result<()> {
        let entry = LogEntry::new(level, message, self.service_id, labels);

        let envelope = TelemetryEnvelope {
            telemetry_type: TelemetryType::Log,
            timestamp: Utc::now(),
            payload: entry.into(),
            metadata: HashMap::new(),
        };

        self.sender
            .send(envelope)
            .await
            .map_err(|e| Error::internal(format!("Failed to send log: {}", e)))?;

        Ok(())
    }

    /// Parse and collect a JSON log line
    pub async fn collect_json(&self, json_line: &str) -> Result<()> {
        let value: serde_json::Value =
            serde_json::from_str(json_line).map_err(|e| Error::Parse {
                message: format!("invalid JSON log: {}", e),
            })?;

        let level = value
            .get("level")
            .and_then(|v| v.as_str())
            .unwrap_or("info")
            .parse()
            .unwrap_or(LogLevel::Info);

        let message = value
            .get("message")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let mut labels = HashMap::new();
        if let Some(obj) = value.as_object() {
            for (k, v) in obj {
                if k != "level" && k != "message" && k != "timestamp" {
                    if let Some(s) = v.as_str() {
                        labels.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        self.collect(level, message, labels).await
    }
}

/// Trace collector - ingests OpenTelemetry spans
pub struct TraceCollector {
    sender: mpsc::Sender<TelemetryEnvelope>,
    service_id: ServiceId,
}

impl TraceCollector {
    /// Create a new trace collector
    pub fn new(producer: Arc<KafkaProducer>, service_id: ServiceId) -> Self {
        let (sender, mut receiver) = mpsc::channel(CHANNEL_BUFFER);
        let producer_clone = producer.clone();

        tokio::spawn(async move {
            while let Some(envelope) = receiver.recv().await {
                if let Err(e) = producer_clone.produce(envelope).await {
                    error!("Failed to produce trace envelope: {}", e);
                }
            }
        });

        Self { sender, service_id }
    }

    /// Collect a trace span
    pub async fn collect_span(&self, span: TraceSpan) -> Result<()> {
        let envelope = TelemetryEnvelope {
            telemetry_type: TelemetryType::Trace,
            timestamp: Utc::now(),
            payload: span.into(),
            metadata: HashMap::new(),
        };

        self.sender
            .send(envelope)
            .await
            .map_err(|e| Error::internal(format!("Failed to send span: {}", e)))?;

        Ok(())
    }

    /// Batch collect multiple spans
    pub async fn collect_spans(&self, spans: Vec<TraceSpan>) -> Result<()> {
        for span in spans {
            self.collect_span(span).await?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_prometheus_line() {
        let service_id = ServiceId::new();
        let collector = MetricsCollector {
            sender: mpsc::channel(1).0,
            service_id,
        };

        // Simple gauge
        let metric = collector.parse_prometheus_line("cpu_usage 75.5").unwrap();
        assert_eq!(metric.name, "cpu_usage");
        assert_eq!(metric.value, 75.5);

        // Counter with labels
        let metric = collector
            .parse_prometheus_line(r#"http_requests_total{method="post",code="200"} 1027"#)
            .unwrap();
        assert_eq!(metric.name, "http_requests_total");
        assert_eq!(metric.metric_type, MetricType::Counter);
        assert_eq!(metric.labels.get("method"), Some(&"post".to_string()));
        assert_eq!(metric.labels.get("code"), Some(&"200".to_string()));
    }

    #[test]
    fn test_parse_labels() {
        let service_id = ServiceId::new();
        let collector = MetricsCollector {
            sender: mpsc::channel(1).0,
            service_id,
        };

        let labels = collector
            .parse_labels(r#"method="post",code="200""#)
            .unwrap();
        assert_eq!(labels.get("method"), Some(&"post".to_string()));
        assert_eq!(labels.get("code"), Some(&"200".to_string()));
    }
}
