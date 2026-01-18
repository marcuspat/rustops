# RustOps Security Architecture

**Document Version**: 1.0
**Date**: 2026-01-18
**Classification**: Internal - Confidential
**Author**: Security Architecture Team

---

## Executive Summary

This document defines the comprehensive security architecture for RustOps, implementing zero-trust principles, defense-in-depth, and secure-by-default patterns. The architecture prioritizes preventing "auto-remediation causing issues" (Critical risk from PRD) through multiple layers of protection, approval gates, and blast radius controls.

**Key Design Principles**:
1. **Never trust, always verify** (Zero Trust)
2. **Explicit approval better than implicit trust** (Human-in-loop)
3. **Failed safe rather than failed dangerous** (Safety interlocks)
4. **Audit everything, encrypt everywhere** (Compliance)

---

## 1. Architecture Overview

### 1.1 Security Domains

```
┌─────────────────────────────────────────────────────────────────────────┐
│                    RUSTOPS SECURITY ARCHITECTURE                        │
├─────────────────────────────────────────────────────────────────────────┤
│                                                                         │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 7: APPLICATION & API SECURITY                  │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │   Axum Web   │  │  GraphQL     │  │  gRPC        │           │  │
│  │  │  Framework   │  │  API         │  │  Services    │           │  │
│  │  │  + Tower     │  │  + Validation│  │  + mTLS      │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                              │                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 6: AUTHENTICATION & AUTHORIZATION             │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │   OAuth2     │  │   OIDC       │  │   RBAC       │           │  │
│  │  │   Provider   │  │   (Keycloak) │  │   Engine     │           │  │
│  │  │  + MFA       │  │  + SAML      │  │  + ABAC      │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                              │                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 5: NETWORK SECURITY & SEGMENTATION             │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │    mTLS      │  │   Service    │  │   Network    │           │  │
│  │  │   (All       │  │   Mesh       │  │   Policies   │           │  │
│  │  │   Services)  │  │  (Istio/     │  │  (Cilium)    │           │  │
│  │  │              │  │   optional)  │  │              │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                              │                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 4: REMEDIATION SECURITY LAYER                 │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │   Approval   │  │   Blast      │  │   Circuit    │           │  │
│  │  │   Gates      │  │   Radius     │  │   Breakers   │           │  │
│  │  │  (Multi-     │  │   Limits     │  │  (Failure    │           │  │
│  │  │   factor)    │  │  (Namespace  │  │   Detection) │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                              │                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 3: DATA SECURITY & ENCRYPTION                  │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │  Encryption  │  │   Secrets    │  │   PII        │           │  │
│  │  │  at Rest     │  │   Management│  │   Redaction  │           │  │
│  │  │  (AES-256    │  │  (HashiCorp  │  │  (Automated  │           │  │
│  │  │   GCM)       │  │   Vault)     │  │   Detection) │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                              │                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 2: INFRASTRUCTURE SECURITY                     │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │  Kubernetes  │  │   Container  │  │   Runtime    │           │  │
│  │  │  RBAC +      │  │   Security   │  │   Security   │           │  │
│  │  │  Pod Security│  │  (Seccomp,   │  │  (Falco,     │           │  │
│  │  │  Policies)   │  │  AppArmor)   │  │  Cilium)     │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                              │                                          │
│  ┌──────────────────────────────────────────────────────────────────┐  │
│  │              LAYER 1: OBSERVABILITY & DETECTION                   │  │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐           │  │
│  │  │   Security   │  │   Audit      │  │   Incident   │           │  │
│  │  │   Logging    │  │   Trail      │  │   Response   │           │  │
│  │  │  (WORM       │  │  (Immutable  │  │  (Automated  │           │  │
│  │  │   Storage)   │  │   Logs)      │  │   Playbooks) │           │  │
│  │  └──────────────┘  └──────────────┘  └──────────────┘           │  │
│  └──────────────────────────────────────────────────────────────────┘  │
│                                                                         │
└─────────────────────────────────────────────────────────────────────────┘
```

### 1.2 Security Perimeters

| Perimeter | Trust Level | Access Control | Monitoring |
|-----------|-------------|----------------|------------|
| **Public Internet** | Untrusted | None, rate-limited | Full logging, WAF |
| **DMZ/API Gateway** | Low | mTLS, API keys | Request/response logging |
| **Application Layer** | Medium | OAuth2/OIDC, RBAC | Activity审计 |
| **Core Services** | High | mTLS, service accounts | Distributed tracing |
| **Data Layer** | Very High | Network isolation, encryption | Query审计 |
| **HSM/Vault** | Critical | Physical security, M-of-N | Complete审计 |

---

## 2. Zero Trust Network Architecture

### 2.1 Principles

1. **Never trust, always verify**: All requests authenticated and authorized
2. **Least privilege**: Minimum required access, time-bound
3. **Assume breach**: Network is hostile, detect lateral movement
4. **Encrypt everywhere**: mTLS for all service-to-service communication

### 2.2 Implementation

#### Mutual TLS (mTLS)

**All inter-service communication requires mTLS**:

```rust
// src/tls/mtls_config.rs
use rustls::ServerConfig;
use rustls_pemfile::{certs, private_key};
use std::io::BufReader;

pub struct MtlsConfig {
    pub ca_cert: Vec<u8>,
    pub server_cert: Vec<u8>,
    pub server_key: Vec<u8>,
}

impl MtlsConfig {
    pub fn build_server_config(&self) -> Result<ServerConfig, TlsError> {
        let mut cert_reader = BufReader::new(&self.server_cert[..]);
        let cert_chain = certs(&mut cert_reader)?
            .into_iter()
            .map(rustls::Certificate)
            .collect();

        let mut key_reader = BufReader::new(&self.server_key[..]);
        let key = private_key(&mut key_reader)?
            .ok_or(TlsError::NoPrivateKey)?;

        let config = ServerConfig::builder()
            .with_safe_defaults()
            .with_no_client_auth()
            .with_single_cert(cert_chain, rustls::PrivateKey(key))?;

        Ok(config)
    }

    /// Enable client certificate verification
    pub fn with_client_auth(&self) -> Result<ServerConfig, TlsError> {
        let mut ca_reader = BufReader::new(&self.ca_cert[..]);
        let ca_certs = certs(&mut ca_reader)?
            .into_iter()
            .map(rustls::Certificate)
            .collect();

        let client_auth_config = rustls::server::ClientCertVerifier::new(
            ca_certs,
            vec![],
            rustls::server::ClientCertVerifier::new(
                rustls::server::WebPkiClientVerifier::new(
                    rustls::server::ClientCertVerifier::builder(
                        rustls::RootCertStore::empty(),
                    )
                    .build()?,
                )
            ),
        );

        // ... rest of configuration
        Ok(config)
    }
}
```

**Certificate Management**:
- **Short-lived certificates**: 24-hour validity
- **Automatic rotation**: Via cert-manager or HashiCorp Vault
- **Revocation**: OCSP stapling or CRL distribution
- **Algorithm**: ECDSA P-384 or Ed25519 (recommended)

#### Network Segmentation

```yaml
# manifests/network-policies.yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: rustops-core-services
  namespace: rustops-system
spec:
  podSelector:
    matchLabels:
      app: rustops-core
  policyTypes:
  - Ingress
  - Egress
  ingress:
  # Only allow mTLS from API Gateway
  - from:
    - namespaceSelector:
        matchLabels:
          name: rustops-gateway
    ports:
    - protocol: TCP
      port: 8443
  egress:
  # Allow database access
  - to:
    - podSelector:
        matchLabels:
          app: postgres
    ports:
    - protocol: TCP
      port: 5432
  # Allow metrics export
  - to:
    - namespaceSelector:
        matchLabels:
          name: monitoring
    ports:
    - protocol: TCP
      port: 9090
```

#### Service Mesh Integration (Optional)

**Istio or Cilium** for advanced features:
- mTLS automation
- Traffic management
- Observability (mutual TLS authentication success/failure)

---

## 3. Authentication & Authorization

### 3.1 Authentication Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                    AUTHENTICATION FLOW                          │
├─────────────────────────────────────────────────────────────────┤
│                                                                 │
│  1. User Request ──▶ API Gateway                                │
│     │                                                            │
│     ▼                                                            │
│  2. API Gateway ──▶ OAuth2 Provider (Keycloak)                  │
│     │                   │                                        │
│     │                   ▼                                        │
│     │              3. Authenticate (MFA)                         │
│     │                   │                                        │
│     │                   ▼                                        │
│     │              4. Issue JWT/ID Token                         │
│     │                   │                                        │
│     ▼                   ▼                                        │
│  5. API Gateway ──▶ Validate Token                             │
│     │                   │                                        │
│     ▼                   ▼                                        │
│  6. Extract Claims  ◄─ JWT (user, groups, expiry)               │
│     │                                                            │
│     ▼                                                            │
│  7. Pass to Authorization                                       │
│                                                                 │
└─────────────────────────────────────────────────────────────────┘
```

### 3.2 OAuth2/OIDC Implementation

```rust
// src/auth/oauth2.rs
use oauth2::{
    AuthorizationCode,
    AuthUrl,
    ClientId,
    ClientSecret,
    CsrfToken,
    PkceCodeChallenge,
    RedirectUrl,
    Scope,
    TokenResponse,
    TokenUrl,
};
use axum::{
    extract::Query,
    response::{Redirect, IntoResponse},
    Extension,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct AuthRequest {
    code: String,
    state: String,
}

pub async fn oauth_callback(
    Query(req): Query<AuthRequest>,
    Extension(oauth_client): Extension<oauth2::basic::BasicClient>,
) -> Result<impl IntoResponse, AuthError> {
    // Exchange code for token
    let token_response = oauth_client
        .exchange_code(AuthorizationCode::new(req.code))
        .request_async(async_http_client)
        .await?;

    // Validate token and extract claims
    let id_token = token_response
        .extra_fields()
        .id_token()
        .ok_or(AuthError::NoIdToken)?;

    let claims = validate_id_token(id_token)?;

    // Create session
    let session = create_session(&claims).await?;

    Ok(Redirect::to("/dashboard"))
}

fn validate_id_token(token: &str) -> Result<TokenClaims, AuthError> {
    // Verify signature (from OAuth2 provider)
    // Verify issuer
    // Verify audience
    // Verify expiry
    // Extract user, groups, email
    todo!("Implement JWT validation using jsonwebtoken crate")
}
```

### 3.3 Multi-Factor Authentication (MFA)

**Required for**:
- All SRE/DevOps users
- Remediation approvals
- Configuration changes
- Admin console access

**Supported MFA methods**:
- TOTP (Time-based One-Time Password)
- WebAuthn (Hardware security keys)
- SMS (Fallback, not recommended)

### 3.4 Role-Based Access Control (RBAC)

```rust
// src/auth/rbac.rs
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Role {
    // Viewer roles (read-only)
    Viewer,
    TopologyViewer,
    LogViewer,

    // Operator roles (basic actions)
    Operator,
    IncidentManager,

    // SRE roles (advanced actions)
    SRE,
    RemediationApprover,

    // Admin roles
    Admin,
    SuperAdmin,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct User {
    pub id: String,
    pub email: String,
    pub roles: HashSet<Role>,
    pub teams: HashSet<String>,
    pub permissions: HashSet<Permission>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Permission {
    // Telemetry permissions
    ReadMetrics,
    ReadLogs,
    ReadTopology,

    // Alert permissions
    ViewAlerts,
    AcknowledgeAlerts,
    SuppressAlerts,

    // Remediation permissions (DANGEROUS)
    ApproveRemediation,
    ExecuteRemediation,
    ApproveDangerousRemediation,

    // Admin permissions
    ModifyConfiguration,
    ManageUsers,
    ManageAgents,
}

impl User {
    pub fn can(&self, permission: Permission) -> bool {
        // Check direct permission
        if self.permissions.contains(&permission) {
            return true;
        }

        // Check role-based permissions
        for role in &self.roles {
            if role.has_permission(permission) {
                return true;
            }
        }

        false
    }

    pub fn can_access_namespace(&self, namespace: &str) -> bool {
        // Super admins have access to all namespaces
        if self.roles.contains(&Role::SuperAdmin) {
            return true;
        }

        // Check team ownership
        self.teams.contains(namespace)
    }

    pub fn can_approve_remediation(&self, risk: RemediationRisk) -> bool {
        match risk {
            RemediationRisk::Low => self.can(Permission::ApproveRemediation),
            RemediationRisk::Medium | RemediationRisk::High => {
                self.can(Permission::ApproveDangerousRemediation)
            }
        }
    }
}

impl Role {
    pub fn has_permission(&self, permission: &Permission) -> bool {
        match self {
            Role::Viewer => matches!(
                permission,
                Permission::ReadMetrics
                    | Permission::ReadLogs
                    | Permission::ReadTopology
                    | Permission::ViewAlerts
            ),
            Role::Operator => matches!(
                permission,
                Permission::ReadMetrics
                    | Permission::ReadLogs
                    | Permission::ReadTopology
                    | Permission::ViewAlerts
                    | Permission::AcknowledgeAlerts
            ),
            Role::SRE => matches!(
                permission,
                Permission::ExecuteRemediation | Permission::ReadMetrics | _
            ),
            Role::RemediationApprover => matches!(
                permission,
                Permission::ApproveRemediation
                    | Permission::ApproveDangerousRemediation
                    | Permission::ExecuteRemediation
            ),
            Role::SuperAdmin => true, // All permissions
            _ => false,
        }
    }
}
```

### 3.5 Remediation Risk Classification

```rust
// src/remediation/risk.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RemediationRisk {
    Low = 1,
    Medium = 2,
    High = 3,
    Critical = 4,
}

impl RemediationRisk {
    pub fn from_action(action: &RemediationAction) -> Self {
        match action {
            // Low risk: Informational actions
            RemediationAction::CreateTicket
            | RemediationAction::SendNotification
            | RemediationAction::AcknowledgeAlert => RemediationRisk::Low,

            // Medium risk: Actions with rollback
            RemediationAction::RestartService
            | RemediationAction::RollbackDeployment
            | RemediationAction::ScaleUp
            | RemediationAction::ScaleDown => RemediationRisk::Medium,

            // High risk: Destructive or infrastructure changes
            RemediationAction::DeletePod
            | RemediationAction::DrainNode
            | RemediationAction::ModifyFirewall
            | RemediationAction::CreateDatabaseSnapshot => RemediationRisk::High,

            // Critical risk: Dangerous or irreversible actions
            RemediationAction::DeleteDatabase
            | RemediationAction::ModifySecurityGroups
            | RemediationAction::DeleteNamespace
            | RemediationAction::RebootInstance => RemediationRisk::Critical,
        }
    }

    pub fn requires_approval(&self) -> bool {
        matches!(self, Self::Medium | Self::High | Self::Critical)
    }

    pub fn approval_count(&self) -> u32 {
        match self {
            Self::Low => 0,
            Self::Medium => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }

    pub fn blast_radius_limit(&self) -> BlastRadiusLimit {
        match self {
            Self::Low => BlastRadiusLimit::SingleService,
            Self::Medium => BlastRadiusLimit::Namespace,
            Self::High => BlastRadiusLimit::Cluster,
            Self::Critical => BlastRadiusLimit::ManualApprovalOnly,
        }
    }
}
```

---

## 4. Secrets Management

### 4.1 HashiCorp Vault Integration

```rust
// src/secrets/vault.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultConfig {
    pub addr: String,
    pub token: String,
    pub mount: String,
}

pub struct VaultClient {
    client: Client,
    config: VaultConfig,
}

impl VaultClient {
    pub fn new(config: VaultConfig) -> Self {
        Self {
            client: Client::new(),
            config,
        }
    }

    /// Retrieve a secret from Vault
    pub async fn get_secret(&self, path: &str) -> Result<VaultSecret, VaultError> {
        let url = format!(
            "{}/v1/{}/{}",
            self.config.addr, self.config.mount, path
        );

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", &self.config.token)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VaultError::RequestFailed(response.status()));
        }

        let secret: VaultSecretResponse = response.json().await?;
        Ok(secret.data)
    }

    /// Store a secret in Vault
    pub async fn store_secret(
        &self,
        path: &str,
        data: VaultSecretData,
    ) -> Result<(), VaultError> {
        let url = format!(
            "{}/v1/{}/{}",
            self.config.addr, self.config.mount, path
        );

        let response = self
            .client
            .post(&url)
            .header("X-Vault-Token", &self.config.token)
            .json(&data)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VaultError::RequestFailed(response.status()));
        }

        Ok(())
    }

    /// Dynamic credentials for database
    pub async fn get_database_credentials(
        &self,
        role: &str,
    ) -> Result<DatabaseCredentials, VaultError> {
        let path = format!("creds/{}", role);
        let secret = self.get_secret(&path).await?;

        Ok(DatabaseCredentials {
            username: secret.data.get("username").cloned().unwrap_or_default(),
            password: secret.data.get("password").cloned().unwrap_or_default(),
        })
    }
}

// Use zeroize to securely clear memory
use zeroize::Zeroize;

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultSecretData {
    #[serde(with = "serde_bytes")]
    value: Vec<u8>,
}

impl Drop for VaultSecretData {
    fn drop(&mut self) {
        self.value.zeroize(); // Securely zero memory
    }
}
```

### 4.2 Secrets Rotation

**Automatic rotation** for:
- Database credentials (24 hours)
- API keys (90 days)
- TLS certificates (24 hours for internal, 90 days for external)
- Service account tokens (hourly for Kubernetes)

```rust
// src/secrets/rotation.rs
use tokio::time::{interval, Duration};

pub struct SecretRotator {
    vault: VaultClient,
    interval: Duration,
}

impl SecretRotator {
    pub async fn start(&self) {
        let mut ticker = interval(self.interval);

        loop {
            ticker.tick().await;

            // Rotate database credentials
            if let Err(e) = self.rotate_database_credentials().await {
                error!("Failed to rotate database credentials: {}", e);
            }

            // Rotate API keys
            if let Err(e) = self.rotate_api_keys().await {
                error!("Failed to rotate API keys: {}", e);
            }

            // Rotate TLS certificates
            if let Err(e) = self.rotate_tls_certificates().await {
                error!("Failed to rotate TLS certificates: {}", e);
            }
        }
    }

    async fn rotate_database_credentials(&self) -> Result<(), RotationError> {
        // For each service, request new credentials from Vault
        // Update service configuration
        // Trigger graceful restart if needed
        todo!("Implement credential rotation")
    }
}
```

---

## 5. Data Encryption

### 5.1 Encryption at Rest

**All data encrypted with AES-256-GCM**:

```rust
// src/crypto/encryption.rs
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

const KEY_SIZE: usize = 32; // 256 bits
const NONCE_SIZE: usize = 12; // 96 bits for GCM

pub struct EncryptionKey([u8; KEY_SIZE]);

impl EncryptionKey {
    pub fn generate() -> Self {
        let mut key = [0u8; KEY_SIZE];
        OsRng.fill_bytes(&mut key);
        EncryptionKey(key)
    }

    pub fn from_vault(vault: &VaultClient, path: &str) -> Result<Self, CryptoError> {
        let secret = vault.get_secret(path).await?;
        let key_bytes = secret.data.get("key").ok_or(CryptoError::KeyNotFound)?;

        if key_bytes.len() != KEY_SIZE {
            return Err(CryptoError::InvalidKeyLength);
        }

        let mut key = [0u8; KEY_SIZE];
        key.copy_from_slice(key_bytes.as_bytes());
        Ok(EncryptionKey(key))
    }
}

pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub fn new(key: EncryptionKey) -> Self {
        let cipher = Aes256Gcm::new(&key.0.into());
        Self { cipher }
    }

    pub fn encrypt(&self, plaintext: &[u8]) -> Result<Vec<u8>, CryptoError> {
        // Generate random nonce
        let nonce = Aes256Gcm::generate_nonce(&mut OsRng);

        // Encrypt
        let ciphertext = self
            .cipher
            .encrypt(&nonce, plaintext)
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Return nonce || ciphertext
        let mut result = Vec::with_capacity(NONCE_SIZE + ciphertext.len());
        result.extend_from_slice(&nonce);
        result.extend_from_slice(&ciphertext);

        Ok(result)
    }

    pub fn decrypt(&self, data: &[u8]) -> Result<Vec<u8>, CryptoError> {
        if data.len() < NONCE_SIZE {
            return Err(CryptoError::InvalidCiphertext);
        }

        // Split nonce and ciphertext
        let (nonce_bytes, ciphertext) = data.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        // Decrypt
        let plaintext = self
            .cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)?;

        Ok(plaintext)
    }
}

// Zeroize key on drop
impl Drop for EncryptionKey {
    fn drop(&mut self) {
        self.0.zeroize();
    }
}
```

### 5.2 Encryption in Transit

**All network traffic encrypted**:
- mTLS for service-to-service (ECDSA P-384)
- TLS 1.3 for external APIs (RSA 2048 or ECDSA P-256)
- VPN for admin access (WireGuard with PSK)

### 5.3 Key Management

**Hierarchical key management**:
```
Root Key (HSM-backed, never leaves HSM)
├── Data Encryption Keys (DEKs) - Rotate quarterly
│   ├── Metrics database DEK
│   ├── Logs database DEK
│   └── ML model store DEK
└── TLS Certificate Keys - Rotate per certificate validity
    ├── API gateway key
    ├── Service mesh CA key
    └── External API keys
```

---

## 6. Audit Logging

### 6.1 Immutable Audit Trail

**Append-only logging using WORM storage**:

```rust
// src/audit/immutable_log.rs
use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuditEvent {
    pub id: uuid::Uuid,
    pub timestamp: DateTime<Utc>,
    pub actor: Actor,
    pub action: AuditAction,
    pub resource: Resource,
    pub result: AuditResult,
    pub ip_address: Option<String>,
    pub user_agent: Option<String>,
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub justification: Option<String>,
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
    // Authentication
    Login,
    Logout,
    MfaChallenge,

    // Authorization
    GrantPermission,
    RevokePermission,

    // Remediation (CRITICAL)
    InitiateRemediation {
        incident_id: String,
        action: String,
        risk: String,
    },
    ApproveRemediation {
        remediation_id: String,
    },
    ExecuteRemediation {
        remediation_id: String,
    },
    RollbackRemediation {
        remediation_id: String,
        reason: String,
    },

    // Configuration
    ModifyConfiguration {
        component: String,
    },
    DeployAgent {
        target: String,
    },

    // Data access
    QueryLogs {
        query: String,
    },
    ExportTopology {
        format: String,
    },
}

pub struct AuditLogger {
    // Append-only storage (WORM)
    storage: ImmutableStorage,
}

impl AuditLogger {
    pub async fn log(&self, event: AuditEvent) -> Result<(), AuditError> {
        // Validate event
        self.validate_event(&event)?;

        // Serialize with canonical JSON (deterministic ordering)
        let json = serde_json::to_string(&event)?;

        // Calculate hash for integrity
        let hash = self.calculate_hash(&json);

        // Append to immutable log
        self.storage.append(hash, json).await?;

        // Forward to SIEM (async, non-blocking)
        tokio::spawn(async move {
            if let Err(e) = forward_to_siem(&event).await {
                error!("Failed to forward audit event to SIEM: {}", e);
            }
        });

        Ok(())
    }

    fn validate_event(&self, event: &AuditEvent) -> Result<(), AuditError> {
        // Validate timestamp is within acceptable window
        let now = Utc::now();
        let max_drift = chrono::Duration::minutes(5);

        if (event.timestamp - now).abs() > max_drift {
            return Err(AuditError::InvalidTimestamp);
        }

        // Validate required fields for dangerous actions
        if matches!(
            event.action,
            AuditAction::ExecuteRemediation { .. } | AuditAction::ApproveRemediation { .. }
        ) {
            if event.justification.is_none() {
                return Err(AuditError::MissingJustification);
            }
        }

        Ok(())
    }

    fn calculate_hash(&self, data: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(data);
        format!("{:x}", hasher.finalize())
    }
}
```

### 6.2 Compliance Logging

**SOC 2 Type II compliant logging**:
- All access to customer data logged
- All remediation actions logged with approval chain
- All configuration changes logged
- Logs retained for 90 days (configurable)
- Logs tamper-evident (cryptographic hashing)

---

## 7. Input Validation & Sanitization

### 7.1 Telemetry Data Validation

```rust
// src/validation/telemetry.rs
use validator::{Validate, ValidationError};
use garde::Validate;
use regex::Regex;

#[derive(Debug, Validate, Deserialize)]
pub struct MetricSubmission {
    #[validate(length(min = 1, max = 100))]
    pub metric_name: String,

    #[validate(custom = "validate_metric_labels")]
    pub labels: std::collections::HashMap<String, String>,

    #[validate(custom = "validate_metric_value")]
    pub value: f64,

    #[validate(range(min = 0, max = i64::MAX))]
    pub timestamp: i64,
}

fn validate_metric_labels(
    labels: &std::collections::HashMap<String, String>,
) -> Result<(), ValidationError> {
    // Validate label names (Prometheus format)
    let label_name_regex = Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$").unwrap();

    for (name, value) in labels {
        // Label name validation
        if !label_name_regex.is_match(name) {
            return Err(ValidationError::new("invalid_label_name"));
        }

        // Label value length limit
        if value.len() > 200 {
            return Err(ValidationError::new("label_value_too_long"));
        }

        // Prevent label cardinality explosion
        if labels.len() > 30 {
            return Err(ValidationError::new("too_many_labels"));
        }
    }

    Ok(())
}

fn validate_metric_value(value: f64) -> Result<(), ValidationError> {
    // Reject NaN and Infinity
    if !value.is_finite() {
        return Err(ValidationError::new("invalid_metric_value"));
    }

    // Reasonable range check
    if value.abs() > 1e20 {
        return Err(ValidationError::new("metric_value_out_of_range"));
    }

    Ok(())
}

// Validate log entries
#[derive(Debug, Validate, Deserialize)]
pub struct LogSubmission {
    #[validate(length(min = 1, max = 4096))]
    pub message: String,

    #[validate(length(max = 64))]
    pub level: String,

    #[validate(length(max = 128))]
    pub service_name: String,

    #[validate(custom = "validate_timestamp")]
    pub timestamp: i64,

    #[serde(default)]
    #[validate(custom = "validate_metadata")]
    pub metadata: serde_json::Value,
}

fn validate_metadata(metadata: &serde_json::Value) -> Result<(), ValidationError> {
    // Prevent deeply nested JSON (DoS prevention)
    fn check_depth(value: &serde_json::Value, depth: u32) -> Result<(), ValidationError> {
        if depth > 10 {
            return Err(ValidationError::new("json_too_deep"));
        }

        match value {
            serde_json::Value::Object(map) => {
                for v in map.values() {
                    check_depth(v, depth + 1)?;
                }
            }
            serde_json::Value::Array(arr) => {
                for v in arr {
                    check_depth(v, depth + 1)?;
                }
            }
            _ => {}
        }

        Ok(())
    }

    check_depth(metadata, 0)
}
```

### 7.2 SQL Injection Prevention

**Use parameterized queries exclusively**:

```rust
// src/database/queries.rs
use sqlx::{PgPool, postgres::PgQueryAs};
use uuid::Uuid;

pub async fn query_logs(
    pool: &PgPool,
    filters: LogQueryFilters,
) -> Result<Vec<LogEntry>, DbError> {
    // Build query with parameterized filters (prevents SQL injection)
    let query = sqlx::query_as::<_, LogEntry>(
        r#"
        SELECT id, timestamp, level, message, service_name, metadata
        FROM logs
        WHERE timestamp >= $1
          AND timestamp <= $2
          AND level = ANY($3)
          AND service_name = ANY($4)
        ORDER BY timestamp DESC
        LIMIT $5
        "#,
    )
    .bind(filters.start_time)
    .bind(filters.end_time)
    .bind(&filters.levels)
    .bind(&filters.service_names)
    .bind(filters.limit);

    let logs = query.fetch_all(pool).await?;
    Ok(logs)
}

// NEVER do this (SQL injection vulnerable):
// let query = format!("SELECT * FROM logs WHERE message LIKE '%{}%'", user_input);
```

### 7.3 SSRF Prevention

**Validate and restrict outbound requests**:

```rust
// src/network/ssrf_protection.rs
use url::Url;
use std::net::{IpAddr, Ipv4Addr, Ipv6Addr};

pub struct RequestValidator {
    // Allowlisted domains
    allowed_domains: Vec<String>,

    // Blocked private IP ranges
    block_private: bool,

    // Block localhost
    block_localhost: bool,
}

impl RequestValidator {
    pub fn validate_url(&self, url_str: &str) -> Result<Url, SSRFError> {
        let url = Url::parse(url_str)?;

        // Block non-HTTP(S) schemes
        if !matches!(url.scheme(), "http" | "https") {
            return Err(SSRFError::InvalidScheme);
        }

        // Resolve hostname to IP
        if let Some(host) = url.host_str() {
            // Check allowlist
            if !self.allowed_domains.is_empty() {
                if !self.allowed_domains.iter().any(|d| host.ends_with(d)) {
                    return Err(SSRFError::DomainNotAllowed);
                }
            }

            // DNS resolution with timeout
            let addrs = tokio::net::lookup_host(host)
                .await
                .map_err(|_| SSRFError::ResolutionFailed)?;

            for addr in addrs {
                let ip = addr.ip();

                // Block private IPs (AWS metadata server, etc.)
                if self.block_private && ip.is_private() {
                    return Err(SSRFError::PrivateIPBlocked);
                }

                // Block localhost
                if self.block_localhost && ip.is_loopback() {
                    return Err(SSRFError::LocalhostBlocked);
                }

                // Block link-local (169.254.169.254 for AWS metadata)
                if ip.is_link_local() {
                    return Err(SSRFError::LinkLocalBlocked);
                }
            }
        }

        Ok(url)
    }
}

// Usage example
pub async fn make_safe_request(url: &str) -> Result<reqwest::Response, SSRFError> {
    let validator = RequestValidator {
        allowed_domains: vec!["api.example.com".to_string()],
        block_private: true,
        block_localhost: true,
    };

    let validated_url = validator.validate_url(url)?;

    // Make request with timeout
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(10))
        .build()?;

    let response = client.get(validated_url).send().await?;
    Ok(response)
}
```

---

## 8. Remediation Safety Layer

### 8.1 Approval Gates

```rust
// src/remediation/approval.rs
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationRequest {
    pub id: uuid::Uuid,
    pub incident_id: String,
    pub action: RemediationAction,
    pub risk: RemediationRisk,
    pub requested_by: Actor,
    pub requested_at: DateTime<Utc>,
    pub justification: String,
    pub blast_radius: BlastRadius,
    pub estimated_impact: ImpactAssessment,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RemediationAction {
    // Low risk
    CreateTicket,
    SendNotification,
    AcknowledgeAlert,

    // Medium risk
    RestartService { service_name: String },
    RollbackDeployment { deployment_id: String },
    ScaleUp { service_name: String, replicas: u32 },
    ScaleDown { service_name: String, replicas: u32 },

    // High risk
    DeletePod { pod_name: String, namespace: String },
    DrainNode { node_name: String },
    ModifyFirewall { rule_id: String },
    CreateDatabaseSnapshot { db_name: String },

    // Critical risk
    DeleteDatabase { db_name: String },
    ModifySecurityGroups { group_id: String },
    DeleteNamespace { namespace: String },
    RebootInstance { instance_id: String },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BlastRadius {
    pub scope: BlastRadiusLimit,
    pub affected_services: Vec<String>,
    pub estimated_users_impacted: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum BlastRadiusLimit {
    SingleService,
    Namespace,
    Cluster,
    ManualApprovalOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ImpactAssessment {
    pub risk_score: u32, // 0-100
    pub potential_downtime_minutes: u32,
    pub potential_data_loss: bool,
    pub rollback_possible: bool,
}

pub struct ApprovalEngine {
    audit: AuditLogger,
    vault: VaultClient,
}

impl ApprovalEngine {
    pub async fn request_approval(
        &self,
        request: RemediationRequest,
    ) -> Result<ApprovalStatus, ApprovalError> {
        // Validate request
        self.validate_request(&request)?;

        // Calculate required approvals
        let required = self.calculate_required_approvals(&request);

        // Create approval record
        let status = ApprovalStatus {
            id: uuid::Uuid::new_v4(),
            remediation_id: request.id,
            required_approvals: required,
            current_approvals: 0,
            approvers: HashMap::new(),
            status: ApprovalState::Pending,
            created_at: Utc::now(),
            expires_at: Utc::now() + chrono::Duration::hours(24),
        };

        // Log request
        self.audit.log(AuditEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: request.requested_by.clone(),
            action: AuditAction::InitiateRemediation {
                incident_id: request.incident_id.clone(),
                action: format!("{:?}", request.action),
                risk: format!("{:?}", request.risk),
            },
            resource: Resource {
                type_: "remediation".to_string(),
                id: request.id.to_string(),
            },
            result: AuditResult::Success,
            ip_address: None,
            user_agent: None,
            request_id: None,
            justification: Some(request.justification.clone()),
        }).await?;

        Ok(status)
    }

    fn validate_request(&self, request: &RemediationRequest) -> Result<(), ApprovalError> {
        // Validate blast radius limits
        match request.risk {
            RemediationRisk::Critical => {
                if !matches!(
                    request.blast_radius.scope,
                    BlastRadiusLimit::ManualApprovalOnly
                ) {
                    return Err(ApprovalError::InvalidBlastRadius);
                }
            }
            RemediationRisk::High => {
                if matches!(
                    request.blast_radius.scope,
                    BlastRadiusLimit::Cluster
                ) {
                    return Err(ApprovalError::BlastRadiusTooLarge);
                }
            }
            _ => {}
        }

        // Validate justification length
        if request.justification.len() < 50 {
            return Err(ApprovalError::InsufficientJustification);
        }

        Ok(())
    }

    fn calculate_required_approvals(&self, request: &RemediationRequest) -> u32 {
        request.risk.approval_count()
    }

    pub async fn approve(
        &self,
        remediation_id: uuid::Uuid,
        approver: &User,
    ) -> Result<ApprovalStatus, ApprovalError> {
        // Load approval status
        let mut status = self.load_status(remediation_id).await?;

        // Validate approver has permission
        if !approver.can_approve_remediation(status.risk) {
            return Err(ApprovalError::Unauthorized);
        }

        // Check for self-approval (must be different person)
        // ...

        // Add approval
        status.approvers.insert(
            approver.id.clone(),
            Approval {
                approver_id: approver.id.clone(),
                approved_at: Utc::now(),
                comments: None,
            },
        );
        status.current_approvals += 1;

        // Check if fully approved
        if status.current_approvals >= status.required_approvals {
            status.status = ApprovalState::Approved;
        }

        // Save and log
        self.save_status(&status).await?;
        self.audit.log(AuditEvent {
            id: uuid::Uuid::new_v4(),
            timestamp: Utc::now(),
            actor: Actor::User {
                id: approver.id.clone(),
                email: approver.email.clone(),
                roles: approver.roles.iter().map(|r| format!("{:?}", r)).collect(),
            },
            action: AuditAction::ApproveRemediation {
                remediation_id: remediation_id.to_string(),
            },
            resource: Resource {
                type_: "approval".to_string(),
                id: status.id.to_string(),
            },
            result: AuditResult::Success,
            ip_address: None,
            user_agent: None,
            request_id: None,
            justification: None,
        }).await?;

        Ok(status)
    }
}
```

### 8.2 Circuit Breakers

```rust
// src/remediation/circuit_breaker.rs
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,   // Normal operation
    Open,     // Failing, stop executing
    HalfOpen, // Testing if recovered
}

pub struct CircuitBreaker {
    state: Arc<RwLock<CircuitState>>,
    failure_count: Arc<RwLock<u32>>,
    failure_threshold: u32,
    success_threshold: u32,
    timeout: std::time::Duration,
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, timeout: std::time::Duration) -> Self {
        Self {
            state: Arc::new(RwLock::new(CircuitState::Closed)),
            failure_count: Arc::new(RwLock::new(0)),
            failure_threshold,
            success_threshold: 3, // Require 3 successes to close circuit
            timeout,
        }
    }

    pub async fn execute<F, T, E>(
        &self,
        operation: F,
    ) -> Result<T, CircuitBreakerError<E>>
    where
        F: std::future::Future<Output = Result<T, E>>,
    {
        // Check state
        let state = *self.state.read().await;

        match state {
            CircuitState::Open => {
                // Check if timeout has elapsed
                return Err(CircuitBreakerError::Open);
            }
            CircuitState::HalfOpen => {
                // Allow one test operation
            }
            CircuitState::Closed => {
                // Normal operation
            }
        }

        // Execute operation
        match operation.await {
            Ok(result) => {
                self.on_success().await;
                Ok(result)
            }
            Err(err) => {
                self.on_failure().await?;
                Err(CircuitBreakerError::Inner(err))
            }
        }
    }

    async fn on_success(&self) {
        let mut state = self.state.write().await;
        let mut failures = self.failure_count.write().await;

        match *state {
            CircuitState::HalfOpen => {
                // Test succeeded, close circuit
                *state = CircuitState::Closed;
                *failures = 0;
            }
            CircuitState::Closed => {
                // Reset failure count
                *failures = 0;
            }
            CircuitState::Open => {
                // Shouldn't happen
            }
        }
    }

    async fn on_failure(&self) -> Result<(), CircuitBreakerError<()>> {
        let mut state = self.state.write().await;
        let mut failures = self.failure_count.write().await;

        *failures += 1;

        if *failures >= self.failure_threshold {
            *state = CircuitState::Open;

            // Schedule transition to HalfOpen after timeout
            let state_clone = self.state.clone();
            let failures_clone = self.failure_count.clone();
            let timeout = self.timeout;

            tokio::spawn(async move {
                tokio::time::sleep(timeout).await;
                let mut s = state_clone.write().await;
                if *s == CircuitState::Open {
                    *s = CircuitState::HalfOpen;
                }
            });

            return Err(CircuitBreakerError::ThresholdExceeded);
        }

        Ok(())
    }
}
```

### 8.3 Rollback Automation

```rust
// src/remediation/rollback.rs
#[derive(Debug, Clone)]
pub struct RollbackPlan {
    pub steps: Vec<RollbackStep>,
    pub estimated_duration: std::time::Duration,
    pub safety_checks: Vec<SafetyCheck>,
}

#[derive(Debug, Clone)]
pub struct RollbackStep {
    pub description: String,
    pub action: RollbackAction,
    pub timeout: std::time::Duration,
}

#[derive(Debug, Clone)]
pub enum RollbackAction {
    UndoRemediation { remediation_id: uuid::Uuid },
    RestoreConfiguration { backup_id: String },
    RestartServices { services: Vec<String> },
    ScaleToPrevious { service: String, replicas: u32 },
    RevertTraffic { from: String, to: String },
}

impl RemediationEngine {
    pub async fn execute_with_rollback(
        &self,
        request: RemediationRequest,
    ) -> Result<RemediationResult, RemediationError> {
        // Generate rollback plan before execution
        let rollback_plan = self.generate_rollback_plan(&request).await?;

        // Execute remediation
        let result = match self.execute_remediation(&request).await {
            Ok(r) => r,
            Err(e) => {
                // Automatic rollback on failure
                warn!(
                    "Remediation {} failed, executing rollback: {}",
                    request.id, e
                );
                self.execute_rollback(rollback_plan).await?;
                return Err(e);
            }
        };

        // Verify success (post-execution checks)
        if let Err(e) = self.verify_remediation(&request, &result).await {
            warn!("Verification failed, executing rollback: {}", e);
            self.execute_rollback(rollback_plan).await?;
            return Err(RemediationError::VerificationFailed);
        }

        Ok(result)
    }

    async fn execute_rollback(
        &self,
        plan: RollbackPlan,
    ) -> Result<(), RemediationError> {
        info!("Executing rollback plan: {} steps", plan.steps.len());

        for step in plan.steps {
            info!("Rollback step: {}", step.description);

            // Execute with timeout
            let result = tokio::time::timeout(
                step.timeout,
                self.execute_rollback_step(step.action),
            )
            .await;

            match result {
                Ok(Ok(())) => {
                    info!("Rollback step succeeded");
                }
                Ok(Err(e)) => {
                    error!("Rollback step failed: {}", e);
                    // Continue with other steps
                }
                Err(_) => {
                    error!("Rollback step timed out");
                    // Continue with other steps
                }
            }
        }

        Ok(())
    }
}
```

---

## 9. Security Monitoring & Incident Response

### 9.1 Security Event Detection

```rust
// src/security/detection.rs
#[derive(Debug, Clone)]
pub enum SecurityEvent {
    // Authentication events
    FailedLogin { user: String, ip: String },
    ImpossibleTravel { user: String, locations: Vec<String> },
    MfaBypass { user: String },

    // Authorization events
    PrivilegeEscalation { user: String, attempted_role: String },
    UnauthorizedAccess { user: String, resource: String },

    // Remediation events (CRITICAL)
    RemediationAbuse { user: String, action: String },
    RemediationFailure { remediation_id: String, reason: String },

    // Data events
    DataExfiltration { user: String, volume_gb: u32 },
    PiiAccess { user: String, data_type: String },

    // Infrastructure events
    AgentCompromise { agent_id: String, anomaly: String },
    MtlsViolation { service: String, reason: String },
}

pub struct SecurityDetector {
    alert_manager: AlertManager,
    audit: AuditLogger,
}

impl SecurityDetector {
    pub async fn detect_suspicious_activity(&self, event: &SecurityEvent) {
        match event {
            SecurityEvent::RemediationAbuse { user, action } => {
                // Critical security event
                self.alert_manager.create_alert(Alert {
                    severity: AlertSeverity::Critical,
                    title: "Potential remediation abuse".to_string(),
                    description: format!(
                        "User {} may be abusing remediation capabilities: {}",
                        user, action
                    ),
                    alert_type: AlertType::SecurityIncident,
                    metadata: serde_json::json!({
                        "user": user,
                        "action": action,
                    }),
                }).await;

                // Immediate security audit
                self.initiate_security_audit(user).await;
            }
            _ => {}
        }
    }

    async fn initiate_security_audit(&self, user: &str) {
        // Lock user account
        // Escalate to security team
        // Create incident ticket
        // Preserve evidence
    }
}
```

### 9.2 Automated Response Playbooks

```rust
// src/security/response.rs
pub struct IncidentResponder {
    vault: VaultClient,
    remediation: RemediationEngine,
}

impl IncidentResponder {
    pub async fn handle_compromised_credentials(&self, user: &str) {
        // 1. Revoke all active sessions for user
        self.revoke_sessions(user).await;

        // 2. Rotate all credentials owned by user
        self.rotate_user_credentials(user).await;

        // 3. Audit all actions by user in last 24 hours
        self.audit_recent_actions(user, chrono::Duration::hours(24)).await;

        // 4. Notify security team
        self.notify_security_team(user).await;

        // 5. Create incident ticket
        self.create_security_incident(user).await;
    }

    pub async fn handle_agent_compromise(&self, agent_id: &str) {
        // 1. Isolate agent from network
        self.isolate_agent(agent_id).await;

        // 2. Revoke agent certificates
        self.revoke_agent_certificates(agent_id).await;

        // 3. Analyze agent for compromise indicators
        self.analyze_agent(agent_id).await;

        // 4. Deploy clean agent
        self.deploy_clean_agent(agent_id).await;

        // 5. Audit all telemetry from compromised agent
        self.audit_agent_telemetry(agent_id).await;
    }
}
```

---

## 10. Compliance & Governance

### 10.1 SOC 2 Type II Compliance

**Audit trail requirements**:
- All access to customer data logged
- All changes to customer data logged
- All remediation actions logged with approval
- Logs tamper-evident (append-only, cryptographic hashing)
- Logs retained for 90 days minimum
- Regular access reviews (quarterly)

**Control implementation**:
- Access control policy (RBAC)
- Change management (approval workflows)
- Data encryption (at rest, in transit)
- Incident response (automated playbooks)
- Vendor management (third-party risk assessment)

### 10.2 GDPR Compliance

**Data subject rights**:
- Right to access: Export all personal data on request
- Right to rectification: Update incorrect personal data
- Right to erasure: Delete personal data on request
- Right to portability: Export in machine-readable format
- Right to object: Stop processing personal data

**Implementation**:
```rust
// src/compliance/gdpr.rs
pub struct GdprHandler {
    db: PgPool,
    audit: AuditLogger,
}

impl GdprHandler {
    pub async fn export_user_data(&self, user_id: &str) -> Result<GdprExport, GdprError> {
        // Collect all personal data from all systems
        let logs = self.get_user_logs(user_id).await?;
        let incidents = self.get_user_incidents(user_id).await?;
        let approvals = self.get_user_approvals(user_id).await?;

        Ok(GdprExport {
            user_id: user_id.to_string(),
            export_date: Utc::now(),
            logs,
            incidents,
            approvals,
        })
    }

    pub async fn delete_user_data(&self, user_id: &str) -> Result<(), GdprError> {
        // Verify deletion request
        // Delete from all systems
        // Confirm deletion
        // Log deletion
        todo!("Implement GDPR right to erasure")
    }
}
```

### 10.3 FedRAMP Compliance

**Security controls** (moderate baseline):
- Access control (AC)
- Awareness and training (AT)
- Audit and accountability (AU)
- System and communications protection (SC)
- System and information integrity (SI)

---

## Appendix A: Security Checklist

### Deployment Checklist
- [ ] mTLS enabled for all services
- [ ] RBAC configured and tested
- [ ] Secrets stored in HashiCorp Vault
- [ ] Audit logging enabled and forwarding to SIEM
- [ ] Network policies applied (deny-all, allow-needed)
- [ ] Pod security policies enforced
- [ ] Runtime security (Falco) deployed
- [ ] Vulnerability scanning completed
- [ ] Penetration testing completed
- [ ] Security review approved

### Operational Checklist
- [ ] TLS certificates rotated within validity period
- [ ] Secrets rotated per schedule
- [ ] Audit logs reviewed daily
- [ ] Security alerts monitored 24/7
- [ ] Incident response runbooks tested
- [ ] Access reviews completed quarterly
- [ ] Security training completed annually
- [ ] Compliance audit completed annually

---

## Appendix B: References

- **NIST Cybersecurity Framework**: https://www.nist.gov/cyberframework
- **CIS Controls**: https://www.cisecurity.org/controls
- **OWASP Top 10**: https://owasp.org/www-project-top-ten/
- **SOC 2**: https://www.aicpa.org/soc4so
- **GDPR**: https://gdpr.eu/
- **FedRAMP**: https://www.fedramp.gov/

---

**Document Status**: Draft for Review
**Next Review**: 2026-02-18
**Approved By**: [Pending Security Review]
