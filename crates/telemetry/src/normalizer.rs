//! # Telemetry normalizer
//!
//! Normalizes telemetry data from various formats into a standard format.

use rustops_common::{Error, LogEntry, Metric, MetricType, Result, ServiceId};
use rustops_common::telemetry::LogLevel;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Telemetry data type (categorizes the kind of telemetry)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryType {
    /// Metric data (Prometheus, OpenTelemetry, etc.)
    Metric,
    /// Log data (JSON, text, Fluentd, etc.)
    Log,
    /// Trace data (Jaeger, Zipkin, OpenTelemetry, etc.)
    Trace,
}

/// Telemetry format variants (source-specific formats)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum TelemetryFormat {
    /// Prometheus metric format
    Prometheus,
    /// OpenTelemetry metric format
    OpenTelemetry,
    /// Fluentd log format
    Fluentd,
    /// JSON log format
    JsonLog,
    /// Text log format
    TextLog,
    /// Jaeger trace format
    Jaeger,
    /// Zipkin trace format
    Zipkin,
}

/// Telemetry normalizer
pub struct TelemetryNormalizer {
    service_id: ServiceId,
}

impl TelemetryNormalizer {
    /// Create a new normalizer
    pub fn new(service_id: ServiceId) -> Self {
        Self { service_id }
    }

    /// Normalize a metric value
    pub fn normalize_metric(&self, raw: &str, format: TelemetryFormat) -> Result<Metric> {
        match format {
            TelemetryFormat::Prometheus => self.normalize_prometheus_metric(raw),
            TelemetryFormat::OpenTelemetry => self.normalize_otel_metric(raw),
            _ => Err(Error::invalid_input("unsupported format for metric")),
        }
    }

    /// Normalize a log entry
    pub fn normalize_log(&self, raw: &str, format: TelemetryFormat) -> Result<LogEntry> {
        match format {
            TelemetryFormat::JsonLog => self.normalize_json_log(raw),
            TelemetryFormat::TextLog => self.normalize_text_log(raw),
            TelemetryFormat::Fluentd => self.normalize_fluentd_log(raw),
            _ => Err(Error::invalid_input("unsupported format for log")),
        }
    }

    /// Normalize Prometheus metric format
    fn normalize_prometheus_metric(&self, raw: &str) -> Result<Metric> {
        // Similar to collector parse logic
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') {
            return Err(Error::invalid_input("empty or comment line"));
        }

        let parts: Vec<&str> = line.split_whitespace().collect();
        if parts.len() < 2 {
            return Err(Error::invalid_input("invalid metric format"));
        }

        let name_part = parts[0];
        let value: f64 = parts[1]
            .parse()
            .map_err(|_| Error::invalid_input("invalid metric value"))?;

        let (name, labels) = if let Some(brace_start) = name_part.find('{') {
            let name = name_part[..brace_start].to_string();
            let labels_str = &name_part[brace_start + 1..name_part.len() - 1];
            (name, self.parse_labels(labels_str)?)
        } else {
            (name_part.to_string(), HashMap::new())
        };

        let metric_type = if name.ends_with("_total") || name.ends_with("_count") {
            MetricType::Counter
        } else if name.ends_with("_bucket") || name.ends_with("_sum") {
            MetricType::Histogram
        } else {
            MetricType::Gauge
        };

        Ok(Metric::new(name, metric_type, value, self.service_id, labels))
    }

    /// Normalize OpenTelemetry metric format
    fn normalize_otel_metric(&self, raw: &str) -> Result<Metric> {
        // Parse OTEL metric JSON format
        let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            Error::Parse {
                message: format!("invalid OTEL metric JSON: {}", e),
            }
        })?;

        let name = value
            .get("name")
            .and_then(|v| v.as_str())
            .ok_or_else(|| Error::invalid_input("missing metric name"))?
            .to_string();

        let gauge = value
            .get("gauge")
            .and_then(|g| g.get("dataPoints"))
            .and_then(|d| d.as_array())
            .and_then(|arr| arr.first())
            .ok_or_else(|| Error::invalid_input("missing gauge data points"))?;

        let val = gauge
            .get("value")
            .and_then(|v| v.as_f64())
            .ok_or_else(|| Error::invalid_input("missing value"))?;

        let mut labels = HashMap::new();
        if let Some(attrs) = gauge.get("attributes") {
            if let Some(obj) = attrs.as_object() {
                for (k, v) in obj {
                    if let Some(s) = v.as_str() {
                        labels.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        Ok(Metric::gauge(name, val, self.service_id, labels))
    }

    /// Normalize JSON log format
    fn normalize_json_log(&self, raw: &str) -> Result<LogEntry> {
        let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            Error::Parse {
                message: format!("invalid JSON log: {}", e),
            }
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
                if k != "level" && k != "message" && k != "timestamp" && k != "time" {
                    if let Some(s) = v.as_str() {
                        labels.insert(k.clone(), s.to_string());
                    }
                }
            }
        }

        Ok(LogEntry::new(level, message, self.service_id, labels))
    }

    /// Normalize text log format
    fn normalize_text_log(&self, raw: &str) -> Result<LogEntry> {
        // Parse common log format: [TIMESTAMP] LEVEL MESSAGE
        let raw = raw.trim();

        // Try to extract level
        let level = if raw.contains("[ERROR]") || raw.contains(" ERROR ") {
            LogLevel::Error
        } else if raw.contains("[WARN]") || raw.contains(" WARN ") {
            LogLevel::Warn
        } else if raw.contains("[INFO]") || raw.contains(" INFO ") {
            LogLevel::Info
        } else if raw.contains("[DEBUG]") || raw.contains(" DEBUG ") {
            LogLevel::Debug
        } else {
            LogLevel::Info
        };

        // Extract message (remove timestamp and level)
        let message = raw
            .split("]")
            .last()
            .unwrap_or(raw)
            .trim()
            .to_string();

        Ok(LogEntry::new(level, message, self.service_id, HashMap::new()))
    }

    /// Normalize Fluentd log format
    fn normalize_fluentd_log(&self, raw: &str) -> Result<LogEntry> {
        // Fluentd in_forward format: JSON with 'log' field
        let value: serde_json::Value = serde_json::from_str(raw).map_err(|e| {
            Error::Parse {
                message: format!("invalid Fluentd format: {}", e),
            }
        })?;

        let log = value
            .get("log")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let level = value
            .get("level")
            .and_then(|v| v.as_str())
            .and_then(|s| s.parse().ok())
            .unwrap_or(LogLevel::Info);

        Ok(LogEntry::new(level, log, self.service_id, HashMap::new()))
    }

    /// Parse labels from label string
    fn parse_labels(&self, labels_str: &str) -> Result<HashMap<String, String>> {
        let mut labels = HashMap::new();

        for pair in labels_str.split(',') {
            let pair = pair.trim();
            if let Some(eq_pos) = pair.find('=') {
                let key = pair[..eq_pos].trim().to_string();
                let value = &pair[eq_pos + 1..];
                let value = value.trim_matches('"').replace("\\\"", "\"");
                labels.insert(key, value);
            }
        }

        Ok(labels)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_normalize_prometheus_metric() {
        let service_id = ServiceId::new();
        let normalizer = TelemetryNormalizer::new(service_id);

        let metric = normalizer
            .normalize_metric(r#"http_requests_total{method="post"} 1027"#, TelemetryFormat::Prometheus)
            .unwrap();

        assert_eq!(metric.name, "http_requests_total");
        assert_eq!(metric.metric_type, MetricType::Counter);
        assert_eq!(metric.labels.get("method"), Some(&"post".to_string()));
    }

    #[test]
    fn test_normalize_json_log() {
        let service_id = ServiceId::new();
        let normalizer = TelemetryNormalizer::new(service_id);

        let json = r#"{"level":"error","message":"Database failed","retry":true}"#;
        let log = normalizer
            .normalize_log(json, TelemetryFormat::JsonLog)
            .unwrap();

        assert_eq!(log.level, LogLevel::Error);
        assert_eq!(log.message, "Database failed");
    }

    #[test]
    fn test_normalize_text_log() {
        let service_id = ServiceId::new();
        let normalizer = TelemetryNormalizer::new(service_id);

        let log = normalizer
            .normalize_log("[ERROR] Database connection failed", TelemetryFormat::TextLog)
            .unwrap();

        assert_eq!(log.level, LogLevel::Error);
        assert_eq!(log.message, "Database connection failed");
    }
}
