# AWS Integration Guide

**Integration Type**: Cloud Infrastructure Monitoring
**Priority**: High (Phase 1)
**Status**: Design

---

## Overview

AWS is the #1 cloud provider (32% market share). RustOps integrates with AWS for comprehensive monitoring of EC2, ECS, Lambda, RDS, and CloudWatch metrics across all AWS services.

### Integration Capabilities

| Capability | Description | Use Case |
|------------|-------------|----------|
| **CloudWatch Metrics** | Pull metrics from CloudWatch | Resource monitoring |
| **CloudWatch Logs** | Stream logs via subscription filters | Log aggregation |
| **EventBridge Events** | Receive real-time AWS events | Change detection |
| **EC2 Operations** | Query instances, start/stop/reboot | Remediation |
| **Lambda Invocation** | Trigger Lambda functions | Auto-remediation |
| **S3 Events** | Object creation/deletion | Data pipeline events |
| **Security Hub** | Aggregate security findings | Security monitoring |

---

## Implementation

### Rust Dependencies

```toml
[dependencies]
# AWS SDK
aws-config = { version = "1.0", features = ["behavior-version-latest"] }
aws-sdk-cloudwatch = "1.0"
aws-sdk-cloudwatchlogs = "1.0"
aws-sdk-ec2 = "1.0"
aws-sdk-lambda = "1.0"
aws-sdk-s3 = "1.0"
aws-sdk-eventbridge = "1.0"

# HTTP client for EventBridge webhooks
reqwest = { version = "0.11", features = ["json", "rustls-tls"] }
```

### AWS Client Setup

```rust
use aws_config::BehaviorVersion;
use aws_sdk_cloudwatch::{Client as CloudWatchClient, Config as CloudWatchConfig};
use aws_sdk_ec2::{Client as Ec2Client};

/// AWS adapter configuration
#[derive(Debug, Clone)]
pub struct AwsConfig {
    pub region: String,
    pub profile: Option<String>,
    pub access_key_id: Option<String>,
    pub secret_access_key: Option<String>,
    pub session_token: Option<String>,
}

/// AWS adapter
pub struct AwsAdapter {
    cloudwatch: CloudWatchClient,
    ec2: Ec2Client,
    region: String,
}

impl AwsAdapter {
    /// Create new AWS adapter
    pub async fn new(config: AwsConfig) -> Result<Self, AwsError> {
        // Load AWS config
        let mut loader = aws_config::defaults(BehaviorVersion::latest());

        if let Some(region) = &config.region {
            loader = loader.region(aws_types::region::Region::new(region.clone()));
        }

        if let Some(profile) = &config.profile {
            loader = loader.profile_name(profile);
        }

        // Use explicit credentials if provided
        if let (Some(key_id), Some(secret)) = (&config.access_key_id, &config.secret_access_key) {
            let creds = aws_types::Credentials::new(
                key_id.clone(),
                secret.clone(),
                config.session_token.clone(),
                None,
                "rustops",
            );
            loader = loader.credentials_provider(creds);
        }

        let sdk_config = loader.load().await;

        let cloudwatch = CloudWatchClient::new(&sdk_config);
        let ec2 = Ec2Client::new(&sdk_config);

        Ok(Self {
            cloudwatch,
            ec2,
            region: config.region,
        })
    }

    /// Health check - verify AWS connectivity
    pub async fn health_check(&self) -> Result<HealthStatus, AwsError> {
        let _ = self.cloudwatch
            .list_metrics()
            .send()
            .await
            .map_err(|e| AwsError::HealthCheck(e.to_string()))?;

        Ok(HealthStatus::Healthy)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AwsError {
    #[error("Config error: {0}")]
    Config(String),

    #[error("Health check failed: {0}")]
    HealthCheck(String),

    #[error("API error: {0}")]
    Api(String),
}

#[derive(Debug, Clone, PartialEq)]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}
```

### CloudWatch Metrics Collection

```rust
use aws_sdk_cloudwatch::types::{Dimension, Statistic};

impl AwsAdapter {
    /// Get CloudWatch metric statistics
    pub async fn get_metric_statistics(
        &self,
        namespace: &str,
        metric_name: &str,
        dimensions: Vec<Dimension>,
        statistic: Statistic,
        period: i64,
        start_time: chrono::DateTime<chrono::Utc>,
        end_time: chrono::DateTime<chrono::Utc>,
    ) -> Result<Vec<MetricDataPoint>, AwsError> {
        let response = self.cloudwatch
            .get_metric_statistics()
            .namespace(namespace)
            .metric_name(metric_name)
            .set_dimensions(Some(dimensions))
            .statistics(statistic)
            .period(period)
            .start_time(start_time)
            .end_time(end_time)
            .send()
            .await
            .map_err(|e| AwsError::Api(e.to_string()))?;

        let datapoints = response.datapoints()
            .unwrap_or(&[])
            .iter()
            .map(|dp| MetricDataPoint {
                timestamp: dp.timestamp().unwrap_or(&chrono::Utc::now()).clone(),
                value: dp.sample_count().unwrap_or(&0.0).clone(),
                statistic: format!("{:?}", statistic),
            })
            .collect();

        Ok(datapoints)
    }

    /// List available metrics
    pub async fn list_metrics(
        &self,
        namespace: &str,
        metric_name: Option<&str>,
        dimensions: Option<Vec<Dimension>>,
    ) -> Result<Vec<CloudWatchMetric>, AwsError> {
        let mut request = self.cloudwatch.list_metrics()
            .namespace(namespace);

        if let Some(name) = metric_name {
            request = request.metric_name(name);
        }

        if let Some(dims) = dimensions {
            request = request.set_dimensions(Some(dims));
        }

        let response = request
            .send()
            .await
            .map_err(|e| AwsError::Api(e.to_string()))?;

        let metrics = response.metrics()
            .unwrap_or(&[])
            .iter()
            .map(|m| CloudWatchMetric {
                namespace: m.namespace().unwrap_or("").to_string(),
                metric_name: m.metric_name().unwrap_or("").to_string(),
                dimensions: m.dimensions().unwrap_or(&[]).to_vec(),
            })
            .collect();

        Ok(metrics)
    }

    /// Get EC2 instance metrics
    pub async fn get_ec2_metrics(
        &self,
        instance_id: &str,
        hours: i64,
    ) -> Result<EC2Metrics, AwsError> {
        let end_time = chrono::Utc::now();
        let start_time = end_time - chrono::Duration::hours(hours);

        let dimensions = vec![
            Dimension::builder()
                .name("InstanceId")
                .value(instance_id)
                .build()
        ];

        // CPU utilization
        let cpu = self.get_metric_statistics(
            "AWS/EC2",
            "CPUUtilization",
            dimensions.clone(),
            Statistic::Average,
            300,  // 5 minute periods
            start_time,
            end_time,
        ).await?;

        // Network in/out
        let net_in = self.get_metric_statistics(
            "AWS/EC2",
            "NetworkIn",
            dimensions.clone(),
            Statistic::Sum,
            300,
            start_time,
            end_time,
        ).await?;

        let net_out = self.get_metric_statistics(
            "AWS/EC2",
            "NetworkOut",
            dimensions,
            Statistic::Sum,
            300,
            start_time,
            end_time,
        ).await?;

        Ok(EC2Metrics {
            instance_id: instance_id.to_string(),
            cpu_utilization: cpu,
            network_in: net_in,
            network_out: net_out,
        })
    }
}

#[derive(Debug, Clone)]
pub struct MetricDataPoint {
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub value: f64,
    pub statistic: String,
}

#[derive(Debug, Clone)]
pub struct CloudWatchMetric {
    pub namespace: String,
    pub metric_name: String,
    pub dimensions: Vec<Dimension>,
}

#[derive(Debug, Clone)]
pub struct EC2Metrics {
    pub instance_id: String,
    pub cpu_utilization: Vec<MetricDataPoint>,
    pub network_in: Vec<MetricDataPoint>,
    pub network_out: Vec<MetricDataPoint>,
}
```

### EC2 Operations

```rust
use aws_sdk_ec2::types::{InstanceType,InstanceStateName};

impl AwsAdapter {
    /// List EC2 instances
    pub async fn list_instances(
        &self,
        filters: Option<Vec<aws_sdk_ec2::types::Filter>>,
    ) -> Result<Vec<EC2Instance>, AwsError> {
        let mut request = self.ec2.describe_instances();

        if let Some(f) = filters {
            request = request.set_filters(Some(f));
        }

        let response = request
            .send()
            .await
            .map_err(|e| AwsError::Api(e.to_string()))?;

        let instances = response.reservations()
            .unwrap_or(&[])
            .iter()
            .flat_map(|r| r.instances().unwrap_or(&[]))
            .map(|i| EC2Instance {
                instance_id: i.instance_id().unwrap_or("").to_string(),
                instance_type: i.instance_type().unwrap_or(&InstanceType::T2Micro).clone(),
                state: i.state().and_then(|s| s.name()).unwrap_or(&InstanceStateName::Unknown).clone(),
                private_ip: i.private_ip_address().map(|s| s.to_string()),
                public_ip: i.public_ip_address().map(|s| s.to_string()),
                tags: i.tags().unwrap_or(&[]).iter()
                    .filter_map(|t| Some((t.key()?.to_string(), t.value()?.to_string())))
                    .collect(),
            })
            .collect();

        Ok(instances)
    }

    /// Reboot EC2 instance
    pub async fn reboot_instance(&self, instance_id: &str) -> Result<(), AwsError> {
        self.ec2.reboot_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .map_err(|e| AwsError::Api(e.to_string()))?;

        tracing::info!("Rebooted EC2 instance: {}", instance_id);
        Ok(())
    }

    /// Stop EC2 instance
    pub async fn stop_instance(&self, instance_id: &str) -> Result<(), AwsError> {
        self.ec2.stop_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .map_err(|e| AwsError::Api(e.to_string()))?;

        tracing::info!("Stopped EC2 instance: {}", instance_id);
        Ok(())
    }

    /// Start EC2 instance
    pub async fn start_instance(&self, instance_id: &str) -> Result<(), AwsError> {
        self.ec2.start_instances()
            .instance_ids(instance_id)
            .send()
            .await
            .map_err(|e| AwsError::Api(e.to_string()))?;

        tracing::info!("Started EC2 instance: {}", instance_id);
        Ok(())
    }
}

#[derive(Debug, Clone)]
pub struct EC2Instance {
    pub instance_id: String,
    pub instance_type: InstanceType,
    pub state: InstanceStateName,
    pub private_ip: Option<String>,
    pub public_ip: Option<String>,
    pub tags: Vec<(String, String)>,
}
```

---

## Configuration

```yaml
integrations:
  aws:
    enabled: true

    # AWS credentials
    credentials:
      # Use default credential chain (env vars, profile, IAM role)
      use_default_chain: true
      # Or explicit credentials
      access_key_id: "${AWS_ACCESS_KEY_ID}"
      secret_access_key: "${AWS_SECRET_ACCESS_KEY}"
      session_token: "${AWS_SESSION_TOKEN}"  # For temporary credentials
      profile: "default"

    # Regions to monitor
    regions:
      - "us-east-1"
      - "us-west-2"
      - "eu-west-1"

    # CloudWatch metrics
    cloudwatch:
      enabled: true
      poll_interval: 60s
      namespaces:
        - "AWS/EC2"
        - "AWS/ECS"
        - "AWS/Lambda"
        - "AWS/RDS"
        - "AWS/ELB"
      metrics:
        CPUUtilization:
          statistic: "Average"
          period: 300
        NetworkIn:
          statistic: "Sum"
          period: 300

    # CloudWatch Logs
    logs:
      enabled: true
      log_groups:
        - "/aws/lambda/*"
        - "/aws/ec2/*"
      subscription_filter_arn: "${CLOUDWATCH_LOGS_ARN}"

    # EventBridge events
    eventbridge:
      enabled: true
      event_pattern:
        source:
          - "aws.ec2"
          - "aws.ecs"
          - "aws.lambda"
        detail_type:
          - "EC2 Instance State-change Notification"
          - "ECS Task State Change"

    # EC2 operations
    ec2:
      enabled: true
      # Allowed actions for auto-remediation
      allowed_actions:
        - reboot
        - stop
        - start
```

---

## References

- [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/)
- [CloudWatch API Reference](https://docs.aws.amazon.com/AmazonCloudWatch/latest/APIReference/)
- [EventBridge API Reference](https://docs.aws.amazon.com/eventbridge/latest/APIReference/)

---

**Version**: 1.0
**Last Updated**: 2026-01-18
**Integration Phase**: Phase 1 (Foundation)
