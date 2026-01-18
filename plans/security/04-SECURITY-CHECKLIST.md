# RustOps Security Checklist

**Document Version**: 1.0
**Date**: 2026-01-18
**Classification**: Internal - Confidential

---

## Component Security Checklist

Use this checklist for each RustOps component before deployment.

### 1. General Security

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **Code Review** | All code reviewed by at least one other engineer | ☐ | PR links |
| **Security Review** | Security-critical code reviewed by security team | ☐ | Review ticket |
| **Threat Modeling** | Threat model created and reviewed | ☐ | Threat model document |
| **Penetration Testing** | External pen test completed | ☐ | Pen test report |
| **Dependencies** | All dependencies audited | ☐ | `cargo audit` output |
| **Licenses** | All licenses compliant | ☐ | `cargo deny check` output |

### 2. Authentication & Authorization

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **MFA** | Multi-factor authentication enforced for all users | ☐ | Config screenshots |
| **Password Policy** | Minimum 12 characters, complexity required | ☐ | Policy document |
| **Password Hashing** | Argon2 with proper parameters | ☐ | Code reference |
| **Session Timeout** | Sessions expire after 24 hours | ☐ | Config values |
| **RBAC** | Role-based access control implemented | ☐ | Role definitions |
| **Approval Gates** | Dangerous actions require approval | ☐ | Approval workflow |
| **Audit Logging** | All auth/authorize actions logged | ☐ | Log samples |

### 3. Input Validation

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **Schema Validation** | All input validated with schema | ☐ | Validation rules |
| **Length Limits** | All strings have max length | ☐ | Validation code |
| **Type Validation** | All types enforced at compile time | ☐ | Type definitions |
| **Cardinality Limits** | Metric labels limited to 30 | ☐ | Validation tests |
| **Rate Limiting** | Per-source rate limiting | ☐ | Rate limiter config |
| **SQL Injection** | All queries parameterized | ☐ | Code review |
| **SSRF Prevention** | URL validation, DNS checks | ☐ | SSRF tests |

### 4. Secrets Management

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **Vault Integration** | All secrets in HashiCorp Vault | ☐ | Vault paths |
| **Secret Rotation** | Automatic rotation enabled | ☐ | Rotation schedule |
| **Encryption at Rest** | AES-256-GCM for data | ☐ | Encryption code |
| **Encryption in Transit** | TLS 1.3 for external, mTLS for internal | ☐ | TLS config |
| **Zeroization** | Sensitive data zeroized in memory | ☐ | Zeroize usage |
| **No Secrets in Logs** | Logs sanitized for secrets | ☐ | Log samples |
| **No Secrets in Code** | No hardcoded secrets | ☐ | Code scan results |

### 5. Network Security

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **mTLS** | All inter-service communication uses mTLS | ☐ | Certificate config |
| **Network Policies** | Kubernetes network policies (deny-all) | ☐ | Network policy YAML |
| **Firewall Rules** | Only necessary ports open | ☐ | Firewall rules |
| **API Gateway** | All external traffic through gateway | ☐ | Gateway config |
| **WAF** | Web Application Firewall enabled | ☐ | WAF rules |
| **DDoS Protection** | Rate limiting at edge | ☐ | Rate limit config |
| **VPN Required** | Admin access requires VPN | ☐ | VPN policy |

### 6. Data Protection

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **PII Redaction** | PII automatically detected and redacted | ☐ | Redaction code |
| **Data Retention** | Logs retained 90 days, then deleted | ☐ | Retention policy |
| **Right to Erasure** | GDPR deletion implemented | ☐ | Erasure endpoint |
| **Data Export** | GDPR export implemented | ☐ | Export endpoint |
| **Backup Encryption** | Backups encrypted at rest | ☐ | Backup procedure |
| **Data Minimization** | Only necessary data collected | ☐ | Data schema |

### 7. Remediation Safety

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **Approval Gates** | Medium+ risk requires approval | ☐ | Approval config |
| **Blast Radius Limits** | Max scope per risk level | ☐ | Blast radius config |
| **Circuit Breakers** | Automatic stop after N failures | ☐ | Circuit breaker code |
| **Rollback Automation** | Automatic rollback on failure | ☐ | Rollback code |
| **Dry Run Mode** | Preview changes before execution | ☐ | Dry run output |
| **Timeouts** | All remediation actions timeout | ☐ | Timeout values |
| **Safety Interlocks** | Manual confirmation for critical actions | ☐ | Interlock tests |

### 8. Monitoring & Logging

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **Audit Trail** | Immutable audit log (WORM) | ☐ | Audit config |
| **Log Forwarding** | Logs forwarded to SIEM | ☐ | SIEM integration |
| **Security Events** | Security events logged separately | ☐ | Security log samples |
| **Alerting** | Security alerts configured | ☐ | Alert rules |
| **Dashboard** | Security dashboard available | ☐ | Dashboard screenshot |
| **Incident Response** | Automated response playbooks | ☐ | Playbook links |

### 9. Compliance

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **SOC 2 Type II** | Audit trail 90 days, access reviews | ☐ | Compliance report |
| **GDPR** | Data subject rights implemented | ☐ | GDPR endpoints |
| **FedRAMP** | Security controls documented | ☐ | Controls matrix |
| **PCI DSS** | Cardholder data protection | ☐ | PCI scan results |
| **Penetration Testing** | Annual pen test completed | ☐ | Pen test report |

### 10. Testing

| Control | Requirement | Status | Evidence |
|---------|-------------|--------|----------|
| **Unit Tests** | Coverage >80% | ☐ | Coverage report |
| **Integration Tests** | All endpoints tested | ☐ | Test results |
| **Security Tests** | SQL injection, XSS, SSRF tested | ☐ | Security test results |
| **Fuzz Testing** | Fuzz tests for parsing code | ☐ | Fuzz test output |
| **Load Testing** | Performance under load verified | ☐ | Load test report |
| **Chaos Testing** | Failure scenarios tested | ☐ | Chaos test results |

---

## Deployment Checklist

Complete this checklist before deploying to production.

### Pre-Deployment

- [ ] Security review approved
- [ ] All tests passing
- [ ] Vulnerability scan clean (0 high/critical)
- [ ] License compliance verified
- [ ] Configuration reviewed
- [ ] Secrets rotated
- [ ] Certificates valid
- [ ] Monitoring configured
- [ ] Alerting configured
- [ ] Rollback plan documented
- [ ] Incident response runbooks updated
- [ ] Stakeholders notified

### Deployment

- [ ] Deploy to canary
- [ ] Monitor canary for 1 hour
- [ ] Verify no errors in logs
- [ ] Verify no security alerts
- [ ] Check performance metrics
- [ ] Approve for full rollout

### Post-Deployment

- [ ] Monitor for 24 hours
- [ ] Review security logs
- [ ] Verify audit trail
- [ ] Check remediation actions
- [ ] Validate compliance
- [ ] Document lessons learned

---

**Document Status**: Draft for Review
**Next Review**: 2026-02-18
**Approved By**: [Pending Security Review]
