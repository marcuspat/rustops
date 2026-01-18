# RustOps Compliance Matrix & Sample Code

**Document Version**: 1.0
**Date**: 2026-01-18
**Classification**: Internal - Confidential

---

## Part 1: Compliance Matrix

### SOC 2 Type II Mapping

| SOC 2 Trust Principle | RustOps Control | Implementation | Evidence |
|----------------------|-----------------|----------------|----------|
| **Security** |  |  |  |
| Access Control | RBAC with MFA | Role-based permissions, multi-factor auth | Policy documents, config |
| Encryption | AES-256-GCM at rest, TLS 1.3 in transit | Encryption crate usage | Code references |
| Audit Logging | Immutable audit trail | WORM storage, hash chaining | Log samples |
| Change Management | Approval workflows | Remediation gates, 4-eyes principle | Workflow configs |
| Incident Response | Automated playbooks | Incident responder code | Playbook files |
| **Availability** |  |  |  |
| Redundancy | Multi-region deployment | Kubernetes clusters across AZs | Infrastructure diagrams |
| Backups | Automated daily backups | Database snapshots, object storage | Backup procedures |
| Monitoring | 24/7 alerting | Prometheus + AlertManager | Alert rules |
| **Processing Integrity** |  |  |  |
| Input Validation | Schema enforcement | garde/validator crates | Validation code |
| Quality Checks | Automated testing | >80% coverage requirement | Test reports |
| Change Detection | Drift monitoring | Terraform drift detection | Drift alerts |
| **Confidentiality** |  |  |  |
| Data Encryption | AES-256-GCM | Encryption at rest | Encryption config |
| PII Protection | Automated redaction | PII detection and masking | Redaction code |
| Access Control | Least privilege | Minimum required permissions | RBAC definitions |
| **Privacy** |  |  |  |
| Data Minimization | Collect only necessary data | Data retention policies | Schema documentation |
| Right to Access | GDPR export endpoint | /api/gdpr/export | API documentation |
| Right to Erasure | GDPR deletion endpoint | /api/gdpr/delete | API documentation |
| Right to Portability | JSON export format | Machine-readable export | API samples |

### GDPR Article 25 (Data Protection by Design)

| Requirement | Implementation | Evidence |
|-------------|----------------|----------|
| Pseudonymization | Hash/token PII fields | Data schema |
| Encryption | Encryption at rest and in transit | Encryption config |
| Access Control | RBAC with justification required | Access logs |
| Data Minimization | Only collect necessary telemetry | Data schema |
| Accuracy | Data validation, error correction | Validation code |
| Storage Limitation | Automatic data deletion after 90 days | Retention policy |
| Integrity | Audit logging, hash chaining | Audit config |
| Confidentiality | Access control, encryption | Security policies |

### FedRAMP Moderate Baseline

| FedRAMP Control | RustOps Implementation | Status |
|----------------|----------------------|--------|
| **AC - Access Control** |  |  |
| AC-1: Access Control Policy | Access control policy documented | Implemented |
| AC-2: Account Management | Automated user provisioning via SSO | Implemented |
| AC-3: Access Enforcement | RBAC enforced at API layer | Implemented |
| AC-6: Least Privilege | Minimum required permissions | Implemented |
| AC-7: Successful/Failure Auditing | All access attempts logged | Implemented |
| AC-11: Session Lock | 24-hour session timeout | Implemented |
| AC-12: Screen Display | No sensitive data in UI logs | Implemented |
| **AU - Audit and Accountability** |  |  |
| AU-2: Audit Events | All CRUD operations logged | Implemented |
| AU-3: Audit Record Content | Timestamp, actor, action, result | Implemented |
| AU-9: Protection of Audit Info | WORM storage, hash chaining | Implemented |
| AU-12: Audit Review | Daily review by security team | Implemented |
| **SC - System and Communications Protection** |  |  |
| SC-7: Boundary Protection | Network segmentation, firewall | Implemented |
| SC-8: Transmission Confidentiality | TLS 1.3, mTLS | Implemented |
| SC-12: Cryptographic Key Management | Vault with HSM backing | Implemented |
| SC-13: Use of Cryptography | AES-256-GCM, RSA 2048+ | Implemented |
| **SI - System and Information Integrity** |  |  |
| SI-1: System Monitoring | Real-time monitoring | Implemented |
| SI-2: Flaw Remediation | Automated vulnerability scanning | Implemented |
| SI-3: Malicious Code Protection | Anti-malware on endpoints | Implemented |
| SI-4: Monitoring for Unauthorized Code | SBOM verification, code signing | Implemented |
| SI-7: Software Firmware Verification | Signed binaries, checksums | Implemented |

---

## Part 2: Sample Secure Rust Code

### Sample 1: Secure API Endpoint with RBAC

```rust
// src/api/remediation.rs
use axum::{
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    Json,
};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Deserialize)]
pub struct RemediationRequest {
    pub incident_id: String,
    pub action: RemediationAction,
    pub justification: String,
}

#[derive(Debug, Serialize)]
pub struct RemediationResponse {
    pub id: Uuid,
    pub status: String,
    pub message: String,
}

// POST /api/v1/remediation
pub async fn create_remediation(
    State(app_state): State<AppState>,
    current_user: CurrentUser, // From JWT middleware
    Json(req): Json<RemediationRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate user has permission
    if !current_user.can(Permission::ExecuteRemediation) {
        return Err(ApiError::Forbidden {
            reason: "User lacks remediation permission".to_string(),
        });
    }

    // Validate request
    req.validate().map_err(|e| ApiError::Validation {
        reason: format!("Invalid request: {}", e),
    })?;

    // Check approval requirement
    let risk = req.action.risk_level();
    if risk.requires_approval() {
        return Ok(Json(RemediationResponse {
            id: Uuid::new_v4(),
            status: "pending_approval".to_string(),
            message: format!(
                "Remediation requires {} approvals",
                risk.approval_count()
            ),
        }));
    }

    // Execute remediation
    let result = app_state
        .remediation_engine
        .execute(req, current_user.id)
        .await?;

    Ok(Json(RemediationResponse {
        id: result.id,
        status: "completed".to_string(),
        message: "Remediation executed successfully".to_string(),
    }))
}

// RBAC implementation
#[derive(Debug, Clone)]
pub struct CurrentUser {
    pub id: String,
    pub email: String,
    pub roles: Vec<Role>,
    pub teams: Vec<String>,
}

impl CurrentUser {
    pub fn can(&self, permission: Permission) -> bool {
        self.roles.iter().any(|role| {
            role.has_permission(permission)
                && self.has_team_access(permission)
        })
    }

    fn has_team_access(&self, permission: Permission) -> bool {
        // Check if permission requires team-level access
        match permission {
            Permission::ExecuteRemediation => {
                // User must have access to target team/namespace
                // This is checked during remediation execution
                true
            }
            _ => true,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum Permission {
    ReadMetrics,
    ReadLogs,
    ExecuteRemediation,
    ApproveRemediation,
    ApproveDangerousRemediation,
}

#[derive(Debug, Clone, Copy)]
pub enum Role {
    Viewer,
    Operator,
    SRE,
    RemediationApprover,
    Admin,
}

impl Role {
    pub fn has_permission(self, permission: Permission) -> bool {
        match (self, permission) {
            (Role::Viewer, Permission::ReadMetrics) => true,
            (Role::Viewer, Permission::ReadLogs) => true,
            (Role::SRE, Permission::ExecuteRemediation) => true,
            (Role::RemediationApprover, Permission::ApproveRemediation) => true,
            (Role::Admin, _) => true,
            _ => false,
        }
    }
}
```

### Sample 2: Immutable Audit Logging

```rust
// src/audit/immutable.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Sha256, Digest};
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor: Actor,
    pub action: AuditAction,
    pub resource: Resource,
    pub result: AuditResult,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub justification: Option<String>,
    pub previous_hash: String,
    pub current_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Actor {
    User {
        id: String,
        email: String,
        roles: Vec<String>,
    },
    Service {
        name: String,
        instance_id: String,
    },
    System {
        component: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditAction {
    Authentication {
        method: String,
        success: bool,
    },
    RemediationInitiated {
        incident_id: String,
        action: String,
        risk: String,
    },
    RemediationApproved {
        remediation_id: String,
        approver_id: String,
    },
    RemediationExecuted {
        remediation_id: String,
        duration_ms: u64,
    },
    RemediationRolledBack {
        remediation_id: String,
        reason: String,
    },
    ConfigurationChanged {
        component: String,
        previous_value: String,
        new_value: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AuditResult {
    Success,
    Failed { reason: String },
    Partial { completed: Vec<String>, failed: Vec<String> },
}

pub struct AuditLogger {
    storage: Arc<dyn ImmutableStorage>,
    last_hash: Arc<RwLock<String>>,
}

#[async_trait::async_trait]
pub trait ImmutableStorage: Send + Sync {
    async fn append(&self, hash: String, event: String) -> Result<(), StorageError>;
    async fn get_latest_hash(&self) -> Result<Option<String>, StorageError>;
}

impl AuditLogger {
    pub async fn log(&self, event: AuditEvent) -> Result<(), AuditError> {
        // Get previous hash for chain integrity
        let previous_hash = {
            let last = self.last_hash.read().await;
            last.clone()
        };

        // Serialize event
        let event_json = serde_json::to_string(&event)?;

        // Calculate current hash
        let current_hash = self.calculate_hash(&previous_hash, &event_json);

        // Create chained event
        let chained_event = AuditEvent {
            previous_hash,
            current_hash: current_hash.clone(),
            ..event
        };

        // Append to immutable storage
        self.storage
            .append(current_hash.clone(), serde_json::to_string(&chained_event)?)
            .await?;

        // Update last hash
        let mut last = self.last_hash.write().await;
        *last = current_hash;

        Ok(())
    }

    fn calculate_hash(&self, previous: &str, event: &str) -> String {
        let mut hasher = Sha256::new();
        hasher.update(previous.as_bytes());
        hasher.update(event.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Verify audit trail integrity
    pub async fn verify_integrity(&self) -> Result<bool, AuditError> {
        let events = self.storage.get_all().await?;

        for (i, event) in events.iter().enumerate() {
            // Verify hash chain
            if i > 0 {
                let previous_event = &events[i - 1];

                // Current event's previous_hash should match previous event's current_hash
                if event.previous_hash != previous_event.current_hash {
                    error!(
                        "Hash chain broken at index {}: expected {}, got {}",
                        i, previous_event.current_hash, event.previous_hash
                    );
                    return Ok(false);
                }
            }

            // Verify current hash
            let event_json = serde_json::to_string(event)?;
            let calculated_hash = if i > 0 {
                self.calculate_hash(&event.previous_hash, &event_json)
            } else {
                // First event
                self.calculate_hash("", &event_json)
            };

            if calculated_hash != event.current_hash {
                error!(
                    "Hash verification failed at index {}: expected {}, got {}",
                    i, event.current_hash, calculated_hash
                );
                return Ok(false);
            }
        }

        Ok(true)
    }
}

// Implementation of immutable storage (append-only S3)
pub struct S3ImmutableStorage {
    bucket: String,
    prefix: String,
    client: aws_sdk_s3::Client,
}

#[async_trait::async_trait]
impl ImmutableStorage for S3ImmutableStorage {
    async fn append(&self, hash: String, event: String) -> Result<(), StorageError> {
        let key = format!("{}/{}.json", self.prefix, hash);

        self.client
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(event.into())
            .send()
            .await?;

        Ok(())
    }

    async fn get_latest_hash(&self) -> Result<Option<String>, StorageError> {
        // List objects and get most recent
        let output = self
            .client
            .list_objects_v2()
            .bucket(&self.bucket)
            .prefix(&self.prefix)
            .max_keys(1)
            .send()
            .await?;

        if let Some(obj) = output.contents().and_then(|objs| objs.first()) {
            let key = obj.key().unwrap();
            let hash = key.rsplit('/').next().unwrap();
            let hash = hash.trim_end_matches(".json");
            Ok(Some(hash.to_string()))
        } else {
            Ok(None)
        }
    }
}
```

### Sample 3: PII Detection and Redaction

```rust
// src/compliance/pii_redaction.rs
use regex::Regex;
use once_cell::sync::Lazy;
use std::collections::HashMap;

// Pre-compiled regex patterns for PII detection
static EMAIL_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b").unwrap()
});

static IP_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d{1,3}\.){3}\d{1,3}\b").unwrap()
});

static CREDIT_CARD_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b(?:\d[ -]*?){13,16}\b").unwrap()
});

static SSN_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{3}-\d{2}-\d{4}\b").unwrap()
});

static PHONE_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b\d{3}[-.]?\d{3}[-.]?\d{4}\b").unwrap()
});

static API_KEY_PATTERN: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"\b[A-Za-z0-9]{32,}\b").unwrap()
});

#[derive(Debug, Clone, Copy)]
pub enum PiiType {
    Email,
    IpAddress,
    CreditCard,
    Ssn,
    PhoneNumber,
    ApiKey,
}

impl PiiType {
    pub fn pattern(&self) -> &Regex {
        match self {
            PiiType::Email => &EMAIL_PATTERN,
            PiiType::IpAddress => &IP_PATTERN,
            PiiType::CreditCard => &CREDIT_CARD_PATTERN,
            PiiType::Ssn => &SSN_PATTERN,
            PiiType::PhoneNumber => &PHONE_PATTERN,
            PiiType::ApiKey => &API_KEY_PATTERN,
        }
    }

    pub fn redaction_value(&self) -> &str {
        match self {
            PiiType::Email => "[REDACTED_EMAIL]",
            PiiType::IpAddress => "[REDACTED_IP]",
            PiiType::CreditCard => "[REDACTED_CC]",
            PiiType::Ssn => "[REDACTED_SSN]",
            PiiType::PhoneNumber => "[REDACTED_PHONE]",
            PiiType::ApiKey => "[REDACTED_API_KEY]",
        }
    }
}

pub struct PiiDetector {
    enabled_types: Vec<PiiType>,
    custom_patterns: HashMap<String, Regex>,
}

impl PiiDetector {
    pub fn new() -> Self {
        Self {
            enabled_types: vec![
                PiiType::Email,
                PiiType::IpAddress,
                PiiType::CreditCard,
                PiiType::Ssn,
                PiiType::PhoneNumber,
                PiiType::ApiKey,
            ],
            custom_patterns: HashMap::new(),
        }
    }

    pub fn with_enabled_types(mut self, types: Vec<PiiType>) -> Self {
        self.enabled_types = types;
        self
    }

    pub fn add_custom_pattern(mut self, name: String, pattern: Regex) -> Self {
        self.custom_patterns.insert(name, pattern);
        self
    }

    /// Detect PII in text
    pub fn detect(&self, text: &str) -> Vec<PiiMatch> {
        let mut matches = Vec::new();

        for pii_type in &self.enabled_types {
            let pattern = pii_type.pattern();
            for mat in pattern.find_iter(text) {
                matches.push(PiiMatch {
                    pii_type: *pii_type,
                    start: mat.start(),
                    end: mat.end(),
                    matched_text: mat.as_str().to_string(),
                });
            }
        }

        // Check custom patterns
        for (name, pattern) in &self.custom_patterns {
            for mat in pattern.find_iter(text) {
                matches.push(PiiMatch {
                    pii_type: PiiType::ApiKey, // Treat custom as sensitive
                    start: mat.start(),
                    end: mat.end(),
                    matched_text: mat.as_str().to_string(),
                });
            }
        }

        matches
    }

    /// Redact PII from text
    pub fn redact(&self, text: &str) -> String {
        let matches = self.detect(text);
        let mut result = text.to_string();

        // Sort matches by start position (reverse order for replacement)
        let mut sorted_matches = matches;
        sorted_matches.sort_by_key(|m| m.start);

        // Build redacted string
        let mut last_end = 0;
        let mut redacted = String::new();

        for mat in sorted_matches {
            // Preserve text before match
            redacted.push_str(&result[last_end..mat.start]);

            // Add redaction
            redacted.push_str(mat.pii_type.redaction_value());

            last_end = mat.end;
        }

        // Add remaining text
        redacted.push_str(&result[last_end..]);

        redacted
    }

    /// Hash PII for privacy-preserving analytics
    pub fn hash_pii(&self, text: &str) -> String {
        use sha2::{Sha256, Digest};

        let matches = self.detect(text);
        let mut result = text.to_string();

        for mat in matches {
            // Hash the matched text
            let mut hasher = Sha256::new();
            hasher.update(mat.matched_text.as_bytes());
            let hashed = format!("{:x}", hasher.finalize());

            // Replace with hash
            result.replace_range(mat.start..mat.end, &hashed[..16]);
        }

        result
    }
}

#[derive(Debug, Clone)]
pub struct PiiMatch {
    pub pii_type: PiiType,
    pub start: usize,
    pub end: usize,
    pub matched_text: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_detection() {
        let detector = PiiDetector::new();
        let text = "Contact user@example.com for support";

        let matches = detector.detect(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Email);
        assert_eq!(matches[0].matched_text, "user@example.com");
    }

    #[test]
    fn test_email_redaction() {
        let detector = PiiDetector::new();
        let text = "Contact user@example.com for support";

        let redacted = detector.redact(text);
        assert_eq!(redacted, "Contact [REDACTED_EMAIL] for support");
    }

    #[test]
    fn test_ip_detection() {
        let detector = PiiDetector::new();
        let text = "Connection from 192.168.1.1 blocked";

        let matches = detector.detect(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::IpAddress);
        assert_eq!(matches[0].matched_text, "192.168.1.1");
    }

    #[test]
    fn test_ssn_detection() {
        let detector = PiiDetector::new();
        let text = "SSN: 123-45-6789";

        let matches = detector.detect(text);
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].pii_type, PiiType::Ssn);
        assert_eq!(matches[0].matched_text, "123-45-6789");
    }

    #[test]
    fn test_multiple_pii() {
        let detector = PiiDetector::new();
        let text = "User john@example.com connected from 10.0.0.1";

        let redacted = detector.redact(text);
        assert!(redacted.contains("[REDACTED_EMAIL]"));
        assert!(redacted.contains("[REDACTED_IP]"));
    }
}
```

### Sample 4: GDPR Implementation

```rust
// src/compliance/gdpr.rs
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct GdprExport {
    pub user_id: String,
    pub export_date: DateTime<Utc>,
    pub logs: Vec<UserLog>,
    pub incidents: Vec<UserIncident>,
    pub approvals: Vec<UserApproval>,
    pub metadata: GdprMetadata,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserLog {
    pub timestamp: DateTime<Utc>,
    pub level: String,
    pub message: String, // PII already redacted
    pub service: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserIncident {
    pub id: Uuid,
    pub title: String,
    pub description: String,
    pub created_at: DateTime<Utc>,
    pub resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserApproval {
    pub id: Uuid,
    pub remediation_id: Uuid,
    pub approved_at: DateTime<Utc>,
    pub comments: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GdprMetadata {
    pub format_version: String,
    pub export_time: DateTime<Utc>,
    pub record_count: usize,
}

pub struct GdprHandler {
    db: sqlx::PgPool,
    audit: AuditLogger,
}

impl GdprHandler {
    /// Export all user data (GDPR Article 15 - Right to Access)
    pub async fn export_user_data(&self, user_id: &str) -> Result<GdprExport, GdprError> {
        // Log the export request
        self.audit.log(AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: Actor::System {
                component: "gdpr_handler".to_string(),
            },
            action: AuditAction::DataExportRequested {
                user_id: user_id.to_string(),
            },
            resource: Resource {
                type_: "user_data".to_string(),
                id: user_id.to_string(),
            },
            result: AuditResult::Success,
            ip_address: None,
            user_agent: None,
            justification: Some("GDPR Article 15 - Right to Access".to_string()),
            previous_hash: String::new(),
            current_hash: String::new(),
        }).await?;

        // Collect all user data
        let logs = self.get_user_logs(user_id).await?;
        let incidents = self.get_user_incidents(user_id).await?;
        let approvals = self.get_user_approvals(user_id).await?;

        let export = GdprExport {
            user_id: user_id.to_string(),
            export_date: Utc::now(),
            logs,
            incidents,
            approvals,
            metadata: GdprMetadata {
                format_version: "1.0".to_string(),
                export_time: Utc::now(),
                record_count: 0, // Calculate
            },
        };

        Ok(export)
    }

    /// Delete all user data (GDPR Article 17 - Right to Erasure)
    pub async fn delete_user_data(&self, user_id: &str) -> Result<(), GdprError> {
        // Verify deletion request (prevent accidental deletion)
        // In production, require explicit confirmation

        // Log the deletion request
        self.audit.log(AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: Actor::System {
                component: "gdpr_handler".to_string(),
            },
            action: AuditAction::DataDeletionRequested {
                user_id: user_id.to_string(),
            },
            resource: Resource {
                type_: "user_data".to_string(),
                id: user_id.to_string(),
            },
            result: AuditResult::Success,
            ip_address: None,
            user_agent: None,
            justification: Some("GDPR Article 17 - Right to Erasure".to_string()),
            previous_hash: String::new(),
            current_hash: String::new(),
        }).await?;

        // Delete from all systems
        // 1. User account
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        // 2. User logs (or anonymize them)
        sqlx::query("DELETE FROM logs WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        // 3. User incidents
        sqlx::query("DELETE FROM incidents WHERE creator_id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        // 4. User approvals
        sqlx::query("DELETE FROM approvals WHERE approver_id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        // 5. Revoke all sessions
        sqlx::query("DELETE FROM sessions WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await?;

        // Log completion
        self.audit.log(AuditEvent {
            id: Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: Actor::System {
                component: "gdpr_handler".to_string(),
            },
            action: AuditAction::DataDeletionCompleted {
                user_id: user_id.to_string(),
            },
            resource: Resource {
                type_: "user_data".to_string(),
                id: user_id.to_string(),
            },
            result: AuditResult::Success,
            ip_address: None,
            user_agent: None,
            justification: None,
            previous_hash: String::new(),
            current_hash: String::new(),
        }).await?;

        Ok(())
    }

    async fn get_user_logs(&self, user_id: &str) -> Result<Vec<UserLog>, GdprError> {
        let logs = sqlx::query_as::<_, UserLog>(
            "SELECT timestamp, level, message, service FROM logs WHERE user_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(logs)
    }

    async fn get_user_incidents(&self, user_id: &str) -> Result<Vec<UserIncident>, GdprError> {
        let incidents = sqlx::query_as::<_, UserIncident>(
            "SELECT id, title, description, created_at, resolved_at FROM incidents WHERE creator_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(incidents)
    }

    async fn get_user_approvals(&self, user_id: &str) -> Result<Vec<UserApproval>, GdprError> {
        let approvals = sqlx::query_as::<_, UserApproval>(
            "SELECT id, remediation_id, approved_at, comments FROM approvals WHERE approver_id = $1"
        )
        .bind(user_id)
        .fetch_all(&self.db)
        .await?;

        Ok(approvals)
    }
}
```

### Sample 5: Remediation Safety Implementation

```rust
// src/remediation/safety.rs
use serde::{Deserialize, Serialize};
use std::time::Duration;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RiskLevel {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl RiskLevel {
    pub fn requires_approval(self) -> bool {
        self >= RiskLevel::Medium
    }

    pub fn approval_count(self) -> u32 {
        match self {
            RiskLevel::Low => 0,
            RiskLevel::Medium => 1,
            RiskLevel::High => 2,
            RiskLevel::Critical => 3,
        }
    }

    pub fn blast_radius_limit(self) -> BlastRadiusLimit {
        match self {
            RiskLevel::Low => BlastRadiusLimit::SingleService,
            RiskLevel::Medium => BlastRadiusLimit::Namespace,
            RiskLevel::High => BlastRadiusLimit::Cluster,
            RiskLevel::Critical => BlastRadiusLimit::ManualApprovalOnly,
        }
    }

    pub fn cooldown_period(self) -> Duration {
        match self {
            RiskLevel::Low => Duration::from_secs(0),
            RiskLevel::Medium => Duration::from_secs(300), // 5 minutes
            RiskLevel::High => Duration::from_secs(1800), // 30 minutes
            RiskLevel::Critical => Duration::from_secs(3600), // 1 hour
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BlastRadiusLimit {
    SingleService,
    Namespace,
    Cluster,
    ManualApprovalOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationAction {
    pub action_type: ActionType,
    pub target: ActionTarget,
    pub parameters: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ActionType {
    // Low risk
    CreateTicket,
    SendNotification,
    AcknowledgeAlert,

    // Medium risk
    RestartService,
    RollbackDeployment,
    ScaleUp,
    ScaleDown,

    // High risk
    DeletePod,
    DrainNode,
    ModifyFirewall,
    CreateDatabaseSnapshot,

    // Critical risk
    DeleteDatabase,
    ModifySecurityGroups,
    DeleteNamespace,
    RebootInstance,
}

impl ActionType {
    pub fn risk_level(&self) -> RiskLevel {
        match self {
            ActionType::CreateTicket
            | ActionType::SendNotification
            | ActionType::AcknowledgeAlert => RiskLevel::Low,

            ActionType::RestartService
            | ActionType::RollbackDeployment
            | ActionType::ScaleUp
            | ActionType::ScaleDown => RiskLevel::Medium,

            ActionType::DeletePod
            | ActionType::DrainNode
            | ActionType::ModifyFirewall
            | ActionType::CreateDatabaseSnapshot => RiskLevel::High,

            ActionType::DeleteDatabase
            | ActionType::ModifySecurityGroups
            | ActionType::DeleteNamespace
            | ActionType::RebootInstance => RiskLevel::Critical,
        }
    }

    pub fn is_dangerous(&self) -> bool {
        self.risk_level() >= RiskLevel::High
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ActionTarget {
    pub namespace: Option<String>,
    pub service: Option<String>,
    pub pod: Option<String>,
    pub node: Option<String>,
    pub cluster: Option<String>,
}

pub struct SafetyValidator {
    max_concurrent_remediations: u32,
    cooldown_tracker: Arc<RwLock<HashMap<String, DateTime<Utc>>>>,
}

impl SafetyValidator {
    pub fn new() -> Self {
        Self {
            max_concurrent_remediations: 5,
            cooldown_tracker: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn validate_remediation(
        &self,
        action: &RemediationAction,
    ) -> Result<SafetyAssessment, SafetyError> {
        let risk = action.action_type.risk_level();
        let mut violations = Vec::new();

        // Check cooldown period
        if let Some(last_remediation) = self.get_last_remediation(&action.target).await {
            let cooldown = risk.cooldown_period();
            let elapsed = Utc::now() - last_remediation;

            if elapsed < cooldown {
                violations.push(SafetyViolation::CooldownPeriodNotElapsed {
                    remaining: cooldown - elapsed,
                });
            }
        }

        // Check blast radius
        let limit = risk.blast_radius_limit();
        if !self.check_blast_radius(&action.target, limit).await? {
            violations.push(SafetyViolation::BlastRadiusTooLarge {
                requested: action.target.clone(),
                limit,
            });
        }

        // Check for concurrent remediations
        if self.count_concurrent_remediations().await? >= self.max_concurrent_remediations {
            violations.push(SafetyViolation::TooManyConcurrentRemediations {
                max: self.max_concurrent_remediations,
            });
        }

        // Check for approval requirement
        if risk.requires_approval() && !self.has_approval(action).await? {
            violations.push(SafetyViolation::ApprovalRequired {
                risk,
                required: risk.approval_count(),
            });
        }

        let is_safe = violations.is_empty();

        Ok(SafetyAssessment {
            is_safe,
            risk,
            violations,
            recommendations: if !is_safe {
                self.generate_recommendations(&violations)
            } else {
                Vec::new()
            },
        })
    }

    async fn get_last_remediation(&self, target: &ActionTarget) -> Option<DateTime<Utc>> {
        let tracker = self.cooldown_tracker.read().await;
        let key = self.target_key(target);
        tracker.get(&key).copied()
    }

    fn target_key(&self, target: &ActionTarget) -> String {
        format!(
            "{}-{}-{}",
            target.namespace.as_deref().unwrap_or("*"),
            target.service.as_deref().unwrap_or("*"),
            target.pod.as_deref().unwrap_or("*")
        )
    }

    async fn check_blast_radius(
        &self,
        target: &ActionTarget,
        limit: BlastRadiusLimit,
    ) -> Result<bool, SafetyError> {
        match limit {
            BlastRadiusLimit::SingleService => {
                // Must specify service, namespace must be set
                Ok(target.service.is_some() && target.namespace.is_some())
            }
            BlastRadiusLimit::Namespace => {
                // Namespace must be set
                Ok(target.namespace.is_some())
            }
            BlastRadiusLimit::Cluster => {
                // Cluster-wide is OK
                Ok(true)
            }
            BlastRadiusLimit::ManualApprovalOnly => {
                // Never auto-approve
                Ok(false)
            }
        }
    }

    async fn count_concurrent_remediations(&self) -> Result<u32, SafetyError> {
        // Query database for active remediations
        todo!("Count active remediations from database")
    }

    async fn has_approval(&self, action: &RemediationAction) -> Result<bool, SafetyError> {
        // Check if action has required approvals
        todo!("Check approval status from database")
    }

    fn generate_recommendations(&self, violations: &[SafetyViolation]) -> Vec<String> {
        violations.iter().map(|v| match v {
            SafetyViolation::CooldownPeriodNotElapsed { remaining } => {
                format!("Wait {} seconds before retrying", remaining.num_seconds())
            }
            SafetyViolation::BlastRadiusTooLarge { limit, .. } => {
                format!("Reduce blast radius to {:?}", limit)
            }
            SafetyViolation::TooManyConcurrentRemediations { max } => {
                format!("Wait for some remediations to complete (max: {})", max)
            }
            SafetyViolation::ApprovalRequired { risk, required } => {
                format!("Get {} approvals for {:?} action", required, risk)
            }
        }).collect()
    }
}

#[derive(Debug, Clone)]
pub struct SafetyAssessment {
    pub is_safe: bool,
    pub risk: RiskLevel,
    pub violations: Vec<SafetyViolation>,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone)]
pub enum SafetyViolation {
    CooldownPeriodNotElapsed { remaining: chrono::Duration },
    BlastRadiusTooLarge { requested: ActionTarget, limit: BlastRadiusLimit },
    TooManyConcurrentRemediations { max: u32 },
    ApprovalRequired { risk: RiskLevel, required: u32 },
}

#[derive(Debug, thiserror::Error)]
pub enum SafetyError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("Safety validation failed: {0}")]
    ValidationFailed(String),
}
```

---

## Testing Checklist

### Unit Tests
- [ ] All validation functions tested
- [ ] All error cases tested
- [ ] Edge cases covered
- [ ] Property-based tests for core logic

### Integration Tests
- [ ] API endpoints tested
- [ ] Database operations tested
- [ ] External integrations tested
- [ ] Error handling verified

### Security Tests
- [ ] Input validation tested with malicious input
- [ ] SQL injection tested
- [ ] SSRF tested
- [ ] Authentication/authorization tested
- [ ] Rate limiting tested
- [ ] Audit logging verified

### Compliance Tests
- [ ] GDPR export tested
- [ ] GDPR deletion tested
- [ ] PII redaction tested
- [ ] Audit trail integrity tested

---

**Document Status**: Draft for Review
**Next Review**: 2026-02-18
**Approved By**: [Pending Security Review]
