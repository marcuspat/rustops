# Integration Documentation Index

This directory contains comprehensive integration guides for the RustOps AIOps platform.

## Integration Architecture

See [README.md](./README.md) for the complete integration strategy including:
- Architecture overview with adapter pattern
- Top 5 prioritized integrations (Phase 1)
- Plugin architecture details
- Testing strategies
- Deployment considerations

---

## Phase 1 Integrations (Foundation)

Top 5 prioritized integrations covering 80% of enterprise use cases.

### Monitoring & Infrastructure

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **Prometheus** | [prometheus-integration.md](./prometheus-integration.md) | Design |
| **Kubernetes** | [kubernetes-integration.md](./kubernetes-integration.md) | Design |

### ITSM & Collaboration

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **ServiceNow** | [servicenow-integration.md](./servicenow-integration.md) | Design |
| **PagerDuty** | [pagerduty-integration.md](./pagerduty-integration.md) | Design |
| **Slack** | [slack-integration.md](./slack-integration.md) | Design |

---

## Phase 2 Integrations (Expansion)

Additional integrations for broader platform support.

### Cloud Platforms

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **AWS** | [aws-integration.md](./aws-integration.md) | Design |
| **Azure** | (pending) | Not Started |
| **GCP** | (pending) | Not Started |

### Issue Tracking

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **Jira** | [jira-integration.md](./jira-integration.md) | Design |

### Observability Platforms

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **Datadog** | [datadog-integration.md](./datadog-integration.md) | Design |
| **Elasticsearch** | [elasticsearch-integration.md](./elasticsearch-integration.md) | Design |
| **Splunk** | (pending) | Not Started |
| **Loki** | (pending) | Not Started |

### Tracing & APM

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **Jaeger** | (pending) | Not Started |
| **Zipkin** | (pending) | Not Started |
| **New Relic** | (pending) | Not Started |

### Collaboration

| Integration | Documentation | Status |
|-------------|---------------|--------|
| **Microsoft Teams** | (pending) | Not Started |
| **OpsGenie** | (pending) | Not Started |

---

## Integration Quick Reference

### Authentication Methods

| Integration | Auth Type | Credential Storage |
|-------------|-----------|-------------------|
| Prometheus | Basic Auth / TLS | K8s Secrets |
| Kubernetes | Service Account | K8s Secrets |
| ServiceNow | OAuth 2.0 | Vault / AWS Secrets |
| PagerDuty | API Token | Vault / AWS Secrets |
| Slack | Bot Token | Vault / AWS Secrets |
| AWS | IAM Key / IRSA | AWS Secrets |
| Jira | API Token | Vault / AWS Secrets |
| Datadog | API Key + App Key | Vault / AWS Secrets |
| Elasticsearch | Basic Auth | Vault / AWS Secrets |

### Rate Limits

| Integration | Default Rate Limit |
|-------------|-------------------|
| Prometheus | 100 req/s |
| Kubernetes | No limit (Watch API) |
| ServiceNow | 10 req/s |
| PagerDuty | 20 req/s |
| Slack | 100 req/min |
| AWS | Varies per service |

### Webhook Support

| Integration | Webhook Support | Purpose |
|-------------|-----------------|---------|
| ServiceNow | Yes | Status updates |
| PagerDuty | Yes | Incident events |
| Slack | Yes | Slash commands |
| AWS | Yes | EventBridge |

---

## API Contract Specifications

See [api-contracts.md](./api-contracts.md) for:
- Unified adapter interface definitions
- API contracts for each integration
- Request/response schemas
- Error handling specifications

---

## Testing Documentation

### Mock Servers

Each integration includes mock server implementations for offline testing:

```rust
#[cfg(test)]
pub mod mock_servicenow {
    pub struct MockServiceNow {
        server: mockito::Server,
    }
    impl MockServiceNow {
        pub fn mock_incident_create(&self) -> Mock;
        pub fn mock_health_check(&self) -> Mock;
    }
}
```

### Contract Testing

Contract tests ensure API compatibility:

```rust
#[tokio::test]
async fn test_servicenow_incident_contract() {
    let mock = MockServiceNow::new();
    let _mock = mock.mock_incident_create();

    let adapter = ServiceNowAdapter::new(...).await;
    let result = adapter.create_incident(...).await;

    assert!(result.is_ok());
}
```

---

## Deployment Guides

### Kubernetes Secrets

Example for ServiceNow credentials:

```yaml
apiVersion: v1
kind: Secret
metadata:
  name: servicenow-credentials
type: Opaque
stringData:
  client_id: "your-client-id"
  client_secret: "your-client-secret"
```

### Environment Variables

```bash
export SERVICENOW_CLIENT_ID="..."
export SERVICENOW_CLIENT_SECRET="..."
export PAGERDUTY_API_TOKEN="..."
export SLACK_BOT_TOKEN="..."
```

---

## Troubleshooting

### Common Issues

| Issue | Integration | Solution |
|-------|-------------|----------|
| Connection timeout | All | Check network, firewall rules |
| 401 Unauthorized | All | Verify credentials/token validity |
| 429 Rate Limit | ServiceNow, Slack | Implement backoff, reduce rate |
| 403 Forbidden | Kubernetes | Create/update RBAC roles |
| Certificate error | Prometheus | Verify TLS configuration |

### Debug Mode

Enable debug logging for integration issues:

```yaml
integrations:
  servicenow:
    debug: true
    log_level: "trace"
```

---

## Additional Resources

### External Documentation

- [Prometheus Documentation](https://prometheus.io/docs/)
- [Kubernetes API Reference](https://kubernetes.io/docs/reference/kubernetes-api/)
- [ServiceNow REST API](https://docs.servicenow.com/bundle/servicenow-platform/page/integrate/inbound-rest/concept/c_RESTAPI.html)
- [PagerDuty API Reference](https://developer.pagerduty.com/api-reference/)
- [Slack API Documentation](https://api.slack.com/docs)
- [AWS SDK for Rust](https://docs.aws.amazon.com/sdk-for-rust/)

### Rust Libraries

- [kube-rs](https://github.com/kube-rs/kube) - Kubernetes client
- [reqwest](https://docs.rs/reqwest/) - HTTP client
- [tokio](https://tokio.rs/) - Async runtime
- [serde](https://serde.rs/) - Serialization

---

## Contributing

When adding a new integration:

1. Create integration guide in `plans/integrations/`
2. Add to this index
3. Implement adapter trait
4. Add mock server for testing
5. Document authentication method
6. Add rate limiting configuration
7. Provide deployment example

---

**Last Updated**: 2026-01-18
