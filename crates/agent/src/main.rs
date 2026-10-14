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
    adapter::{MetricQuery, TelemetryCollector},
    telemetry::prometheus::{PrometheusAdapter, PrometheusConfig},
    IntegrationAdapter,
};
use rustops_telemetry::{KafkaProducer, MetricsCollector};
use std::collections::HashMap;
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

impl AgentConfig {
    /// Load configuration from the environment, falling back to
    /// defaults for anything unset or malformed.
    ///
    /// - `PROMETHEUS_URL` — Prometheus server URL; accepted only if it
    ///   parses as an absolute http(s) URL with a host (e.g.
    ///   `http://prometheus:9090`). Anything else is rejected without
    ///   being logged, so a malformed value can't leak credentials
    ///   into the startup log
    /// - `PROMETHEUS_INTERVAL_SECS` — scrape interval in seconds
    fn from_env() -> Self {
        let mut config = Self::default();

        if let Ok(url) = std::env::var("PROMETHEUS_URL") {
            let url = url.trim().to_string();
            // Parse before accepting: a prefix check alone would let
            // garbage like "http://" (no host) or "https://host:invalid"
            // (bad port) replace the default, and a schemeless
            // "user:pass@host" could carry credentials past the
            // redactor into the startup log. Store the *normalized*
            // serialization — Url::parse accepts noncanonical forms
            // like "http:user:pass@host", and the redactor's string
            // matching assumes "scheme://". Rejecting without echoing
            // the value keeps bad input out of the log; reqwest needs
            // an absolute URL anyway.
            match reqwest::Url::parse(&url) {
                Ok(parsed)
                    if parsed.has_host()
                        && (parsed.scheme() == "http" || parsed.scheme() == "https") =>
                {
                    config.prometheus_url = parsed.to_string();
                }
                _ => warn!("PROMETHEUS_URL is not a valid http(s) URL; using default"),
            }
        }

        if let Ok(raw) = std::env::var("PROMETHEUS_INTERVAL_SECS") {
            if let Ok(secs) = raw.trim().parse::<u64>() {
                if secs > 0 {
                    config.interval_secs = secs;
                } else {
                    warn!("PROMETHEUS_INTERVAL_SECS must be > 0; using default");
                }
            } else {
                warn!(
                    "PROMETHEUS_INTERVAL_SECS={} is not a number; using default",
                    raw
                );
            }
        }

        config
    }
}

/// Telemetry collection agent
struct Agent {
    config: AgentConfig,
    prometheus: PrometheusAdapter,
    collector: MetricsCollector,
}

impl Agent {
    /// Create a new agent
    fn new(config: AgentConfig) -> Result<Self, rustops_common::Error> {
        // Create Prometheus adapter
        let prometheus = PrometheusAdapter::new(PrometheusConfig {
            url: config.prometheus_url.clone(),
            username: None,
            password: None,
            bearer_token: None,
            timeout: Duration::from_secs(30),
        });

        // Create Kafka producer (stub for now)
        let producer = Arc::new(KafkaProducer::new(config.service_id).map_err(|e| {
            rustops_common::Error::internal(format!("Failed to create producer: {}", e))
        })?);

        // Create metrics collector (holds the producer handle)
        let collector = MetricsCollector::new(producer, config.service_id);

        Ok(Self {
            config,
            prometheus,
            collector,
        })
    }

    /// Initialize the agent
    async fn initialize(&mut self) -> Result<(), rustops_common::Error> {
        info!("Initializing RustOps telemetry agent");
        info!(
            "Prometheus URL: {}",
            redact_url_credentials(&self.config.prometheus_url)
        );
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

            // One clock read for both ends of the window (two Utc::now()
            // calls made the window drift by the loop latency), and the
            // step = min(interval, 15s): never longer than the window (so
            // sub-15s intervals never ask for a step beyond their range) and
            // never longer than 15s (matching the previous fixed step).
            let now = Utc::now();
            let interval = self.config.interval_secs.max(1);
            let metric_query = MetricQuery {
                metric_name: query.clone(),
                labels: HashMap::new(),
                start_time: now - chrono::Duration::seconds(interval as i64),
                end_time: now,
                step: Some(interval.min(15)),
            };

            match self.prometheus.collect_metrics(metric_query).await {
                Ok(metrics) => {
                    info!("Query '{}' returned {} samples", query, metrics.len());

                    for metric in metrics {
                        // Convert to Prometheus text-format line with the real
                        // sampled value.
                        let labels: Vec<String> = metric
                            .labels
                            .iter()
                            .map(|(k, v)| format!(r#"{}="{}""#, k, v))
                            .collect();

                        let line = if labels.is_empty() {
                            format!("{} {}", metric.name, metric.value)
                        } else {
                            format!(
                                r#"{}{{{}}} {}"#,
                                metric.name,
                                labels.join(","),
                                metric.value
                            )
                        };

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

/// Strip any `user:password@` userinfo from a URL before it reaches a
/// log line. URLs like `http://user:pass@prometheus:9090` would leak
/// credentials into logs. Parses when possible (covering noncanonical
/// forms like `http:user:pass@host` that Url accepts); falls back to a
/// conservative authority match for unparseable strings. Malformed or
/// userinfo-free URLs pass through unchanged.
fn redact_url_credentials(url: &str) -> String {
    if let Ok(parsed) = reqwest::Url::parse(url) {
        if parsed.has_host() && (parsed.scheme() == "http" || parsed.scheme() == "https") {
            let mut redacted = parsed;
            let had_userinfo = !redacted.username().is_empty() || redacted.password().is_some();
            if had_userinfo {
                let _ = redacted.set_username("***");
                let _ = redacted.set_password(None);
                return redacted.to_string();
            }
            return url.to_string();
        }
    }
    // Fallback: only the authority (between "://" and the first '/',
    // '?' or '#') can carry userinfo; an '@' elsewhere belongs to the
    // URL and must not trigger redaction.
    if let Some(scheme_end) = url.find("://") {
        let rest = &url[scheme_end + 3..];
        let authority_end = rest.find(['/', '?', '#']).unwrap_or(rest.len());
        let authority = &rest[..authority_end];
        if let Some(at) = authority.rfind('@') {
            let scheme = &url[..scheme_end + 3];
            return format!("{scheme}***@{}", &rest[at + 1..]);
        }
    }
    url.to_string()
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

    // Load configuration from the environment (PROMETHEUS_URL,
    // PROMETHEUS_INTERVAL_SECS; defaults for anything unset)
    let config = AgentConfig::from_env();
    info!(
        "Agent config: prometheus={} interval={}s",
        redact_url_credentials(&config.prometheus_url),
        config.interval_secs
    );

    // Create and initialize agent
    let mut agent = Agent::new(config)?;
    agent.initialize().await?;

    // Spawn the agent run loop in a task; a oneshot lets us stop the loop
    // and still run the agent's graceful shutdown path afterwards.
    let (shutdown_tx, shutdown_rx) = tokio::sync::oneshot::channel::<()>();
    let agent_task = tokio::spawn(async move {
        tokio::select! {
            result = agent.run() => {
                if let Err(e) = result {
                    error!("Agent run loop failed: {}", e);
                }
            }
            _ = shutdown_rx => {}
        }
        if let Err(e) = agent.shutdown().await {
            error!("Agent shutdown failed: {}", e);
        }
    });

    // Wait for shutdown signal
    wait_for_shutdown().await;

    info!("Shutdown signal received, stopping agent...");

    // Stop the run loop and wait for graceful shutdown to finish
    let _ = shutdown_tx.send(());
    let _ = agent_task.await;

    info!("Agent stopped");

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// One sequential test: these scenarios mutate process-global env
    /// vars, so parallel siblings would race each other.
    #[test]
    fn from_env_defaults_overrides_and_malformed_inputs() {
        std::env::remove_var("PROMETHEUS_URL");
        std::env::remove_var("PROMETHEUS_INTERVAL_SECS");
        let config = AgentConfig::from_env();
        assert_eq!(config.prometheus_url, "http://localhost:9090");
        assert_eq!(config.interval_secs, 15);

        std::env::set_var("PROMETHEUS_URL", "http://prometheus.internal:9091");
        std::env::set_var("PROMETHEUS_INTERVAL_SECS", "30");
        let config = AgentConfig::from_env();
        assert_eq!(config.prometheus_url, "http://prometheus.internal:9091");
        assert_eq!(config.interval_secs, 30);

        // Noncanonical-but-parseable input is accepted and normalized,
        // so the redactor can always find the userinfo.
        std::env::set_var("PROMETHEUS_URL", "http:user:pass@prometheus:9090");
        let config = AgentConfig::from_env();
        assert_eq!(config.prometheus_url, "http://user:pass@prometheus:9090");
        assert_eq!(
            redact_url_credentials(&config.prometheus_url),
            "http://***@prometheus:9090",
            "accepted noncanonical URL must redact through config"
        );

        std::env::set_var("PROMETHEUS_INTERVAL_SECS", "fast");
        let config = AgentConfig::from_env();
        assert_eq!(
            config.interval_secs, 15,
            "malformed interval must fall back"
        );

        std::env::set_var("PROMETHEUS_INTERVAL_SECS", "0");
        let config = AgentConfig::from_env();
        assert_eq!(config.interval_secs, 15, "zero interval must fall back");

        std::env::set_var("PROMETHEUS_URL", "   ");
        let config = AgentConfig::from_env();
        assert_eq!(
            config.prometheus_url, "http://localhost:9090",
            "whitespace-only URL must fall back"
        );

        // A schemeless value could embed credentials ("user:pass@host")
        // that the URL redactor cannot recognize without a scheme; it
        // must be rejected outright, never logged or used. Degenerate
        // http(s) values (no host, invalid port) must not replace the
        // default either.
        for bad in [
            "user:pass@host:9090",
            "http://",
            "https://host:invalid",
            "ftp://prometheus:9090",
        ] {
            std::env::set_var("PROMETHEUS_URL", bad);
            let config = AgentConfig::from_env();
            assert_eq!(
                config.prometheus_url, "http://localhost:9090",
                "malformed URL {bad:?} must fall back to default"
            );
        }

        std::env::remove_var("PROMETHEUS_URL");
        std::env::remove_var("PROMETHEUS_INTERVAL_SECS");
    }

    #[test]
    fn redact_url_credentials_strips_userinfo() {
        assert_eq!(
            redact_url_credentials("http://user:pass@prometheus:9090"),
            "http://***@prometheus:9090"
        );
        // Url::parse accepts noncanonical scheme-special forms without
        // the double slash; the parse-based redaction must still hide
        // their credentials.
        assert_eq!(
            redact_url_credentials("http:user:pass@prometheus:9090"),
            "http://***@prometheus:9090",
            "noncanonical accepted-by-parse URLs must redact"
        );
        assert_eq!(
            redact_url_credentials("http://user:pass@prometheus:9090/api/v1"),
            "http://***@prometheus:9090/api/v1",
            "host and path must survive redaction"
        );
        assert_eq!(
            redact_url_credentials("https://alice@prom.example.com/api"),
            "https://***@prom.example.com/api"
        );
        assert_eq!(
            redact_url_credentials("http://localhost:9090"),
            "http://localhost:9090",
            "userinfo-free URLs must pass through unchanged"
        );
        assert_eq!(
            redact_url_credentials("not a url"),
            "not a url",
            "malformed input must pass through, not panic"
        );
        assert_eq!(
            redact_url_credentials("http://host/path?x=user@example.com"),
            "http://host/path?x=user@example.com",
            "'@' in query must not trigger redaction"
        );
        assert_eq!(
            redact_url_credentials("http://host?q=user@example.com"),
            "http://host?q=user@example.com",
            "query before any '/' must not be treated as authority"
        );
        assert_eq!(
            redact_url_credentials("http://host#user@example.com"),
            "http://host#user@example.com",
            "fragment must not be treated as authority"
        );
        assert_eq!(
            redact_url_credentials("user:pass@host:9090"),
            "user:pass@host:9090",
            "no scheme separator: not a URL, pass through"
        );
        assert_eq!(
            redact_url_credentials("http://user:pass@host:9090?q=1#frag"),
            "http://***@host:9090?q=1#frag",
            "query and fragment must survive authority redaction"
        );
    }
}
