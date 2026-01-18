# RustOps Secure Rust Patterns

**Document Version**: 1.0
**Date**: 2026-01-18
**Classification**: Internal - Confidential
**Author**: Security Architecture Team

---

## Executive Summary

This document catalogs secure coding patterns in Rust for the RustOps AIOps platform. Each pattern includes: description, security rationale, Rust implementation, and common pitfalls.

**Core Principles**:
1. **Leverage Rust's type system** for compile-time guarantees
2. **Never panic in production code** (use Result/Option)
3. **Zeroize sensitive data** (prevent memory leaks)
4. **Validate all input** (trust nothing)
5. **Use crates with security audits** (vet dependencies)

---

## 1. Input Validation Patterns

### 1.1 Validation with `garde` (Recommended)

**Why**: `garde` provides compile-time checked validation rules with custom validators.

```rust
// Cargo.toml
[dependencies]
garde = { version = "0.20", features = ["derive"] }
regex = "1.10"

// src/validation/models.rs
use garde::Validate;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Validate, Deserialize, Serialize)]
pub struct MetricSubmission {
    #[garde(length(min = 1, max = 100))]
    #[garde(custom(validate_metric_name))]
    pub name: String,

    #[garde(custom(validate_labels))]
    pub labels: std::collections::HashMap<String, String>,

    #[garde(custom(validate_metric_value))]
    pub value: f64,

    #[garde(range(min = 0, max = i64::MAX))]
    pub timestamp: i64,
}

// Custom validator for metric names
fn validate_metric_name(name: &str, _: &()) -> garde::Result {
    let regex = regex::Regex::new(r"^[a-zA-Z_:][a-zA-Z0-9_:]*$")
        .map_err(|_| garde::Error::new("invalid_regex"))?;

    if !regex.is_match(name) {
        return Err(garde::Error::new("invalid_metric_name_format"));
    }

    if name.starts_with("__") {
        return Err(garde::Error::new("reserved_prefix"));
    }

    Ok(())
}

// Custom validator for labels
fn validate_labels(
    labels: &std::collections::HashMap<String, String>,
    _: &(),
) -> garde::Result {
    if labels.len() > 30 {
        return Err(garde::Error::new("too_many_labels"));
    }

    let label_regex = regex::Regex::new(r"^[a-zA-Z_][a-zA-Z0-9_]*$")
        .map_err(|_| garde::Error::new("invalid_regex"))?;

    for (name, value) in labels {
        if !label_regex.is_match(name) {
            return Err(garde::Error::new("invalid_label_name"));
        }

        if value.len() > 200 {
            return Err(garde::Error::new("label_value_too_long"));
        }

        if is_high_cardinality_value(value) {
            return Err(garde::Error::new("high_cardinality_value"));
        }
    }

    Ok(())
}

fn is_high_cardinality_value(value: &str) -> bool {
    // Detect UUIDs
    if uuid::Uuid::parse_str(value).is_ok() {
        return true;
    }

    // Detect ISO timestamps
    if value.len() >= 20 && value.contains('T') && value.contains('Z') {
        return true;
    }

    // Detect hex strings
    if value.len() > 32 && value.chars().all(|c| c.is_ascii_hexdigit()) {
        return true;
    }

    false
}

// Custom validator for metric values
fn validate_metric_value(value: &f64, _: &()) -> garde::Result {
    if !value.is_finite() {
        return Err(garde::Error::new("value_must_be_finite"));
    }

    if value.abs() > 1e20 {
        return Err(garde::Error::new("value_out_of_range"));
    }

    Ok(())
}
```

### 1.2 Validation with `validator` (Alternative)

**Why**: `validator` is widely used, has good documentation.

```rust
// Cargo.toml
[dependencies]
validator = { version = "0.18", features = ["derive"] }

// src/validation/validator_models.rs
use validator::{Validate, ValidationError};

#[derive(Debug, Clone, Validate, Deserialize)]
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

fn validate_timestamp(timestamp: &i64) -> Result<(), ValidationError> {
    let now = chrono::Utc::now().timestamp();

    // Reject timestamps > 5 minutes in future (clock skew)
    if *timestamp > now + 300 {
        return Err(ValidationError::new("timestamp_in_future"));
    }

    // Reject timestamps > 90 days old (data retention)
    if *timestamp < now - 86400 * 90 {
        return Err(ValidationError::new("timestamp_too_old"));
    }

    Ok(())
}

fn validate_metadata(metadata: &serde_json::Value) -> Result<(), ValidationError> {
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

---

## 2. Secrets Handling Patterns

### 2.1 Using `secrecy` (Secret Wrapper)

**Why**: `secrecy` provides type-based secrecy with:
- No accidental logging or serialization
- Zeroization on drop
- Clone protection

```rust
// Cargo.toml
[dependencies]
secrecy = { version = "0.10", features = ["serde"] }
zeroize = "1.7"

// src/secrets/secret_types.rs
use secrecy::{Secret, ExposeSecret};
use zeroize::Zeroize;

#[derive(Clone, serde::Serialize, serde::Deserialize)]
pub struct DatabaseCredentials {
    pub username: String,
    #[serde(with = "secrecy::serde")]
    pub password: Secret<String>,
}

impl DatabaseCredentials {
    pub fn new(username: String, password: String) -> Self {
        Self {
            username,
            password: Secret::new(password),
        }
    }

    // Expose password only when absolutely necessary
    pub fn password(&self) -> &str {
        self.password.expose_secret()
    }
}

// Example: Secure database connection
use sqlx::PgPool;

pub async fn connect_to_db(creds: DatabaseCredentials) -> Result<PgPool, DbError> {
    let connection_string = format!(
        "postgresql://{}:{}@localhost/mydb",
        creds.username,
        creds.password.expose_secret() // Only expose here
    );

    let pool = PgPool::connect(&connection_string).await?;
    Ok(pool)
}
```

### 2.2 Using `zeroize` (Secure Memory Clearing)

**Why**: `zeroize` securely clears memory, preventing data leakage in core dumps.

```rust
// Cargo.toml
[dependencies]
zeroize = { version = "1.7", features = ["serde"] }

// src/secrets/zeroize_types.rs
use zeroize::{Zeroize, ZeroizeOnDrop};

#[derive(ZeroizeOnDrop)]
pub struct ApiKey {
    key: String,
}

impl ApiKey {
    pub fn new(key: String) -> Self {
        Self { key }
    }

    pub fn as_str(&self) -> &str {
        &self.key
    }
}

// Example: Encryption key that zeroizes on drop
use aes_gcm::Aes256Gcm;

#[derive(ZeroizeOnDrop)]
pub struct EncryptionKey([u8; 32]);

impl EncryptionKey {
    pub fn generate() -> Self {
        let mut key = [0u8; 32];
        getrandom::getrandom(&mut key).expect("RNG failed");
        EncryptionKey(key)
    }

    pub fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

// Manual zeroization
pub fn process_sensitive_data(data: &mut Vec<u8>) {
    // Process data
    perform_crypto_operation(data);

    // Securely zero memory
    data.zeroize();
}
```

### 2.3 HashiCorp Vault Integration

```rust
// Cargo.toml
[dependencies]
reqwest = { version = "0.12", features = ["json"] }
serde = { version = "1.0", features = ["derive"] }

// src/secrets/vault.rs
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct VaultSecret {
    pub data: std::collections::HashMap<String, String>,
}

pub struct VaultClient {
    client: Client,
    addr: String,
    token: Secret<String>,
}

impl VaultClient {
    pub fn new(addr: String, token: String) -> Self {
        Self {
            client: Client::new(),
            addr,
            token: Secret::new(token),
        }
    }

    pub async fn get_secret(&self, path: &str) -> Result<VaultSecret, VaultError> {
        let url = format!("{}/v1/secret/data/{}", self.addr, path);

        let response = self
            .client
            .get(&url)
            .header("X-Vault-Token", self.token.expose_secret())
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(VaultError::RequestFailed(response.status()));
        }

        let secret: VaultSecret = response.json().await?;
        Ok(secret)
    }
}
```

---

## 3. Cryptographic Patterns

### 3.1 Encryption (AES-256-GCM)

**Why**: AES-256-GCM provides authenticated encryption (confidentiality + integrity).

```rust
// Cargo.toml
[dependencies]
aes-gcm = "0.10"
rand = "0.8"

// src/crypto/encryption.rs
use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;

const NONCE_SIZE: usize = 12; // 96 bits for GCM

pub struct Encryptor {
    cipher: Aes256Gcm,
}

impl Encryptor {
    pub fn new(key: &[u8; 32]) -> Self {
        let cipher = Aes256Gcm::new(key.into());
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
```

### 3.2 Password Hashing (Argon2)

**Why**: Argon2 is the winner of the Password Hashing Competition (2015).

```rust
// Cargo.toml
[dependencies]
argon2 = "0.5"
rand = "0.8"

// src/crypto/password_hashing.rs
use argon2::{
    password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
    Argon2,
};

pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    let salt = SaltString::generate(&mut rand::rngs::OsRng);
    let argon2 = Argon2::default();

    let password_hash = argon2
        .hash_password(password.as_bytes(), &salt)
        .map_err(|_| PasswordError::HashingFailed)?;

    Ok(password_hash.to_string())
}

pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    let parsed_hash = PasswordHash::new(hash)
        .map_err(|_| PasswordError::InvalidHash)?;

    Ok(Argon2::default()
        .verify_password(password.as_bytes(), &parsed_hash)
        .is_ok())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hash_and_verify() {
        let password = "secure_password_123";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("wrong_password", &hash).unwrap());
    }
}
```

### 3.3 Key Derivation (HKDF)

```rust
// Cargo.toml
[dependencies]
hkdf = "0.12"
sha2 = "0.10"

// src/crypto/key_derivation.rs
use hkdf::Hkdf;
use sha2::Sha256;

pub fn derive_key(
    ikm: &[u8], // Input key material
    salt: &[u8],
    info: &[u8],
    output: &mut [u8],
) -> Result<(), CryptoError> {
    let hk = Hkdf::<Sha256>::new(Some(salt), ikm);
    hk.expand(info, output)
        .map_err(|_| CryptoError::KeyDerivationFailed)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_key_derivation() {
        let ikm = b"0123456789ABCDEF0123456789ABCDEF";
        let salt = b"salt_value";
        let info = b"key_derivation";

        let mut okm1 = [0u8; 32];
        let mut okm2 = [0u8; 32];

        derive_key(ikm, salt, info, &mut okm1).unwrap();
        derive_key(ikm, salt, info, &mut okm2).unwrap();

        // Same inputs should produce same outputs
        assert_eq!(okm1, okm2);
    }
}
```

---

## 4. Error Handling Patterns

### 4.1 Never Panic in Production

**Why**: Panics cause process crashes. Use `Result` and `Option` for recoverable errors.

```rust
// src/errors/mod.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RustOpsError {
    #[error("Database error: {0}")]
    Database(#[from] sqlx::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Validation error: {0}")]
    Validation(String),

    #[error("Authentication failed")]
    AuthenticationFailed,

    #[error("Authorization denied: {0}")]
    AuthorizationDenied(String),

    #[error("Remediation failed: {0}")]
    RemediationFailed(String),
}

// Always return Result, never unwrap/expect in production
pub async fn process_metric(
    metric: MetricSubmission,
) -> Result<ProcessedMetric, RustOpsError> {
    // Validate
    metric.validate()
        .map_err(|e| RustOpsError::Validation(format!("{:?}", e)))?;

    // Process
    let processed = ProcessedMetric::from(metric).await?;

    Ok(processed)
}

// NEVER do this:
// let metric = parse_metric(data).unwrap(); // Will crash on error

// ALWAYS do this:
// let metric = parse_metric(data)?;
// OR
// let metric = parse_metric(data).unwrap_or_else(|e| {
//     error!("Failed to parse metric: {}", e);
//     return Err(e.into());
// });
```

### 4.2 Custom Error Types

```rust
// src/errors/custom.rs
use thiserror::Error;

#[derive(Debug, Error)]
pub enum RemediationError {
    #[error("Approval required for {risk:?} action")]
    ApprovalRequired { risk: RemediationRisk },

    #[error("Blast radius too large: {limit:?}")]
    BlastRadiusTooLarge { limit: BlastRadiusLimit },

    #[error("Remediation timed out after {timeout:?}")]
    Timeout { timeout: std::time::Duration },

    #[error("Rollback failed: {reason}")]
    RollbackFailed { reason: String },
}

// Convert to HTTP status codes
impl actix_web::error::ResponseError for RemediationError {
    fn error_response(&self) -> HttpResponse {
        match self {
            RemediationError::ApprovalRequired { .. } => {
                HttpResponse::Forbidden().json(json!({
                    "error": "approval_required",
                    "message": self.to_string(),
                }))
            }
            RemediationError::BlastRadiusTooLarge { .. } => {
                HttpResponse::BadRequest().json(json!({
                    "error": "blast_radius_too_large",
                    "message": self.to_string(),
                }))
            }
            _ => HttpResponse::InternalServerError().json(json!({
                "error": "internal_error",
                "message": self.to_string(),
            })),
        }
    }
}
```

---

## 5. Concurrency Patterns

### 5.1 Channel-based Communication

**Why**: Channels prevent race conditions and data races.

```rust
// src/concurrency/channels.rs
use tokio::sync::mpsc;
use std::time::Duration;

#[derive(Debug, Clone)]
pub enum TelemetryMessage {
    Metric(MetricSubmission),
    Log(LogSubmission),
    Trace(TraceSubmission),
}

pub struct TelemetryProcessor {
    metrics_tx: mpsc::Sender<TelemetryMessage>,
}

impl TelemetryProcessor {
    pub fn new(buffer_size: usize) -> Self {
        let (metrics_tx, mut metrics_rx) = mpsc::channel(buffer_size);

        // Spawn background task
        tokio::spawn(async move {
            while let Some(msg) = metrics_rx.recv().await {
                match msg {
                    TelemetryMessage::Metric(metric) => {
                        if let Err(e) = process_metric(metric).await {
                            error!("Failed to process metric: {}", e);
                        }
                    }
                    TelemetryMessage::Log(log) => {
                        if let Err(e) = process_log(log).await {
                            error!("Failed to process log: {}", e);
                        }
                    }
                    TelemetryMessage::Trace(trace) => {
                        if let Err(e) = process_trace(trace).await {
                            error!("Failed to process trace: {}", e);
                        }
                    }
                }
            }
        });

        Self { metrics_tx }
    }

    pub async fn send(&self, msg: TelemetryMessage) -> Result<(), ChannelError> {
        // Timeout after 5 seconds
        tokio::time::timeout(
            Duration::from_secs(5),
            self.metrics_tx.send(msg),
        )
        .await
        .map_err(|_| ChannelError::Timeout)?
        .map_err(|_| ChannelError::Closed)?;

        Ok(())
    }
}
```

### 5.2 Async Mutex (not std::sync::Mutex)

**Why**: `std::sync::Mutex` causes deadlocks in async code. Use `tokio::sync::Mutex`.

```rust
// src/concurrency/async_mutex.rs
use tokio::sync::Mutex;
use std::sync::Arc;

pub struct SharedState {
    // Use Arc<Mutex<T>> for shared mutable state
    counter: Arc<Mutex<u64>>,
}

impl SharedState {
    pub fn new() -> Self {
        Self {
            counter: Arc::new(Mutex::new(0)),
        }
    }

    pub async fn increment(&self) -> u64 {
        let mut counter = self.counter.lock().await;
        *counter += 1;
        *counter
    }

    pub async fn get(&self) -> u64 {
        *self.counter.lock().await
    }
}

// NEVER use std::sync::Mutex in async code
// use std::sync::Mutex; // WRONG - causes deadlocks

// ALWAYS use tokio::sync::Mutex
use tokio::sync::Mutex; // CORRECT
```

### 5.3 Semaphore for Rate Limiting

```rust
// src/concurrency/semaphore.rs
use tokio::sync::Semaphore;
use std::sync::Arc;

pub struct RateLimiter {
    semaphore: Arc<Semaphore>,
}

impl RateLimiter {
    pub fn new(max_concurrent: usize) -> Self {
        Self {
            semaphore: Arc::new(Semaphore::new(max_concurrent)),
        }
    }

    pub async fn acquire(&self) -> SemaphorePermit<'_> {
        self.semaphore.acquire().await.unwrap()
    }
}

// Usage
pub async fn process_with_limit(
    limiter: &RateLimiter,
    item: Item,
) -> Result<(), Error> {
    // Acquire permit (blocks if limit reached)
    let _permit = limiter.acquire().await;

    // Process item
    process_item(item).await?;

    Ok(())
}
```

---

## 6. Network Security Patterns

### 6.1 Timeout All Network Requests

```rust
// src/network/timeouts.rs
use reqwest::Client;
use std::time::Duration;

pub struct HttpClient {
    client: Client,
}

impl HttpClient {
    pub fn new() -> Result<Self, reqwest::Error> {
        let client = Client::builder()
            .timeout(Duration::from_secs(10)) // Overall timeout
            .connect_timeout(Duration::from_secs(5)) // Connection timeout
            .pool_idle_timeout(Duration::from_secs(90))
            .pool_max_idle_per_host(10)
            .build()?;

        Ok(Self { client })
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client.get(url).send().await
    }

    pub async fn post_json<T: serde::Serialize>(
        &self,
        url: &str,
        body: &T,
    ) -> Result<reqwest::Response, reqwest::Error> {
        self.client
            .post(url)
            .json(body)
            .timeout(Duration::from_secs(30))
            .send()
            .await
    }
}
```

### 6.2 mTLS Client

```rust
// src/network/mtls.rs
use reqwest::Client;
use rustls::ClientConfig;

pub struct MtlsClient {
    client: Client,
}

impl MtlsClient {
    pub fn new(cert_pem: &[u8], key_pem: &[u8]) -> Result<Self, MtlsError> {
        // Build mTLS config
        let mut cert_reader = std::io::BufReader::new(cert_pem);
        let cert_chain = rustls_pemfile::certs(&mut cert_reader)?
            .into_iter()
            .map(rustls::Certificate)
            .collect();

        let mut key_reader = std::io::BufReader::new(key_pem);
        let key = rustls_pemfile::private_key(&mut key_reader)?
            .ok_or(MtlsError::NoPrivateKey)?;

        let config = ClientConfig::builder()
            .with_safe_defaults()
            .with_root_certificates(root_certs)
            .with_single_cert(cert_chain, rustls::PrivateKey(key))?;

        let client = Client::builder()
            .use_preconfigured_tls(config)
            .build()?;

        Ok(Self { client })
    }

    pub async fn get(&self, url: &str) -> Result<reqwest::Response, reqwest::Error> {
        self.client.get(url).send().await
    }
}
```

---

## 7. Dependency Security

### 7.1 Cargo Audit Configuration

```toml
# .cargo/audit.toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/RustSec/advisory-db"]

[vulnerabilities]
alert = "yolo" # Treat all vulnerabilities as alert
informational = "warn"
severity = "medium"

[output]
deny = ["warnings"]
format = "terminal"
quiet = false
```

### 7.2 Cargo Deny Configuration

```toml
# deny.toml
[advisories]
db-path = "~/.cargo/advisory-db"
db-urls = ["https://github.com/RustSec/advisory-db"]
vulnerability = "deny"
unmaintained = "warn"
yanked = "warn"
notice = "warn"

[licenses]
unlicensed = "deny"
allow-osi-fsf-free = "both"
copyleft = "warn"
default = "deny"

[[licenses.clarify]]
name = "ring"
expression = "ISC AND MIT AND OpenSSL"
license-files = [
    { path = "LICENSE", hash = 0xbd0eed23 }
]

[bans]
multiple-versions = "warn"
wildcards = "allow"
highlight = "all"

[sources]
unknown-registry = "warn"
unknown-git = "warn"
allow-registry = ["https://github.com/rust-lang/crates.io-index"]
allow-git = []
```

---

## 8. Testing Patterns

### 8.1 Property-Based Testing

```rust
// Cargo.toml
[dev-dependencies]
proptest = "1.4"

// tests/property_tests.rs
use proptest::prelude::*;

proptest! {
    #[test]
    fn test_metric_validation_roundtrip(name in "[a-zA-Z_][a-zA-Z0-9_]*") {
        let metric = MetricSubmission {
            name: name.clone(),
            labels: std::collections::HashMap::new(),
            value: 42.0,
            timestamp: chrono::Utc::now().timestamp(),
        };

        // Should validate successfully
        prop_assert!(metric.validate().is_ok());

        // Should serialize/deserialize correctly
        let json = serde_json::to_string(&metric).unwrap();
        let deserialized: MetricSubmission = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(metric.name, deserialized.name);
    }

    #[test]
    fn test_high_cardinality_detection(value in "[0-9a-f-]{36}") {
        // UUID-like strings should be flagged
        prop_assert!(is_high_cardinality_value(&value));
    }
}
```

### 8.2 Fuzz Testing

```rust
// Cargo.toml
[dev-dependencies]
cargo-fuzz = "0.11"

// fuzz/fuzz_targets/metric_parsing.rs
#![no_main]
use libfuzzer_sys::fuzz_target;

fuzz_target!(|data: &[u8]| {
    if let Ok(s) = std::str::from_utf8(data) {
        let _ = parse_metric(s);
    }
});
```

---

## 9. Checklist for Secure Rust Code

### Before Committing
- [ ] No `.unwrap()` or `.expect()` in production code paths
- [ ] All external input validated
- [ ] All secrets wrapped in `Secret<T>` or zeroized
- [ ] All network requests have timeouts
- [ ] All SQL queries parameterized
- [ ] All errors propagated (not ignored)
- [ ] All sensitive data cleared from memory
- [ ] Dependencies audited (`cargo audit`)
- [ ] License compliance verified (`cargo deny check`)

### Before Deploying
- [ ] Security review completed
- [ ] Penetration testing passed
- [ ] Vulnerability scan passed
- [ ] Code coverage >80%
- [ ] fuzz testing completed
- [ ] threat modeling reviewed

---

## 10. References

- **The Rust Book**: https://doc.rust-lang.org/book/
- **Rust Secure Coding**: https://github.com/rust-secure-code
- **RustSec Advisories**: https://github.com/RustSec/advisory-db
- **cargo-audit**: https://github.com/RustSec/cargo-audit
- **cargo-deny**: https://github.com/EmbarkStudios/cargo-deny

---

**Document Status**: Draft for Review
**Next Review**: 2026-02-18
**Approved By**: [Pending Security Review]
