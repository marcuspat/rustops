# Hooks and Worker Integration

**Automation and Background Processing for Memory Operations**

## Overview

RustOps integrates deeply with Claude Flow's hooks system for automated memory operations during incident resolution, and uses 12 background workers for continuous optimization and learning.

---

## Hooks Integration

### 1. pre-task Hook

**Purpose**: Route incidents to optimal remediation agents based on learned patterns.

```bash
#!/bin/bash
# hooks/pre-task.sh
# Route incident to optimal agent with SONA mode selection

set -euo pipefail

INCIDENT_ID="${1}"
INCIDENT_TYPE="${2}"
SERVICE="${3}"
ENVIRONMENT="${4}"
SEVERITY="${5}"

# Call Claude Flow pre-task hook
OUTPUT=$(npx @claude-flow/cli@latest hooks pre-task \
  --task-id "$INCIDENT_ID" \
  --description "Incident: $INCIDENT_TYPE affecting $SERVICE in $ENVIRONMENT (Severity: $SEVERITY)" \
  --coordinate-swarm 2>/dev/null)

# Extract routing recommendation
AGENT_TYPE=$(echo "$OUTPUT" | jq -r '.suggested_agents[0].type')
SONA_MODE=$(echo "$OUTPUT" | jq -r '.sona_mode')
COMPLEXITY=$(echo "$OUTPUT" | jq -r '.complexity')

echo "{
  \"incident_id\": \"$INCIDENT_ID\",
  \"routed_to\": \"$AGENT_TYPE\",
  \"sona_mode\": \"$SONA_MODE\",
  \"complexity\": $COMPLEXITY,
  \"reason\": \"Optimal agent selection based on incident characteristics and learned patterns\"
}"
```

**Example Output**:

```json
{
  "incident_id": "01HZ...",
  "routed_to": "incident-resolver",
  "sona_mode": "balanced",
  "complexity": 0.7,
  "reason": "Optimal agent selection based on incident characteristics and learned patterns"
}
```

### 2. post-task Hook

**Purpose**: Store successful patterns for reuse after incident resolution.

```bash
#!/bin/bash
# hooks/post-task.sh
# Store learning from resolved incident

set -euo pipefail

INCIDENT_ID="${1}"
SUCCESS="${2}"  # true/false
QUALITY_SCORE="${3:-0.5}"  # 0-1
DURATION_SECONDS="${4}"

if [ "$SUCCESS" != "true" ]; then
  echo "Incident not successful, skipping pattern storage"
  exit 0
fi

# Only store if quality is above threshold
if (( $(echo "$QUALITY_SCORE < 0.7" | bc -l) )); then
  echo "Quality score below threshold, skipping pattern storage"
  exit 0
fi

# Store pattern in memory
npx @claude-flow/cli@latest hooks post-task \
  --task-id "$INCIDENT_ID" \
  --success "$SUCCESS" \
  --quality-score "$QUALITY_SCORE" \
  --store-patterns \
  --train-neural 2>/dev/null

# Extract and store pattern
PATTERN_ID=$(npx @claude-flow/cli@latest memory store \
  --namespace "patterns" \
  --key "pattern-incident-$INCIDENT_ID" \
  --value "{\"incident_id\":\"$INCIDENT_ID\",\"quality\":$QUALITY_SCORE,\"duration\":$DURATION_SECONDS}" \
  --tags "extracted-from-incident,quality-$(echo "$QUALITY_SCORE" | awk '{printf "%.0f", $1*100}')")

echo "{
  \"incident_id\": \"$INCIDENT_ID\",
  \"pattern_stored\": true,
  \"pattern_id\": \"$PATTERN_ID\",
  \"neural_trained\": true
}"
```

### 3. post-edit Hook

**Purpose**: Learn from configuration changes and track impact.

```bash
#!/bin/bash
# hooks/post-edit.sh
# Track configuration changes for learning

set -euo pipefail

CONFIG_FILE="${1}"
OPERATION="${2}"  # create/update/delete
USER="${3:-unknown}"

# Parse service from file path
SERVICE=$(echo "$CONFIG_FILE" | sed -n 's/.*services/\([^/]*\).*/\1/p')

# Store config change
npx @claude-flow/cli@latest memory store \
  --namespace "config-changes" \
  --key "config-$(date +%s)-$(basename "$CONFIG_FILE")" \
  --value "{
    \"file\": \"$CONFIG_FILE\",
    \"operation\": \"$OPERATION\",
    \"service\": \"$SERVICE\",
    \"user\": \"$USER\",
    \"timestamp\": \"$(date -u +%Y-%m-%dT%H:%M:%SZ)\"
  }" \
  --tags "config-change,service-$SERVICE,operation-$OPERATION" 2>/dev/null

# Start monitoring for impact
echo "{
  \"config_file\": \"$CONFIG_FILE\",
  \"monitoring_started\": true,
  \"impact_window\": \"15m\"
}"
```

### 4. worker-dispatch Hook

**Purpose**: Trigger background workers based on events.

```bash
#!/bin/bash
# hooks/worker-dispatch.sh
# Dispatch background workers for various triggers

set -euo pipefail

TRIGGER="${1}"
CONTEXT="${2:-}"
PRIORITY="${3:-normal}"

case "$TRIGGER" in
  "ultralearn")
    # Deep knowledge acquisition after major incident
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "ultralearn" \
      --context "$CONTEXT" \
      --priority "$PRIORITY" \
      --background 2>/dev/null
    ;;

  "optimize")
    # Performance optimization after remediation
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "optimize" \
      --context "$CONTEXT" \
      --priority "$PRIORITY" \
      --background 2>/dev/null
    ;;

  "consolidate")
    # Memory consolidation (low priority)
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "consolidate" \
      --context "$CONTEXT" \
      --priority "low" \
      --background 2>/dev/null
    ;;

  "audit")
    # Security analysis for remediation actions
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "audit" \
      --context "$CONTEXT" \
      --priority "$PRIORITY" \
      --background 2>/dev/null
    ;;

  "testgaps")
    # Find missing test coverage
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "testgaps" \
      --context "$CONTEXT" \
      --priority "normal" \
      --background 2>/dev/null
    ;;

  "map")
    # Update codebase map after major changes
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "map" \
      --context "$CONTEXT" \
      --priority "normal" \
      --background 2>/dev/null
    ;;

  "deepdive")
    # Deep code analysis
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "deepdive" \
      --context "$CONTEXT" \
      --priority "$PRIORITY" \
      --background 2>/dev/null
    ;;

  "document")
    # Auto-documentation
    npx @claude-flow/cli@latest hooks worker-dispatch \
      --trigger "document" \
      --context "$CONTEXT" \
      --priority "low" \
      --background 2>/dev/null
    ;;

  *)
    echo "Unknown trigger: $TRIGGER"
    exit 1
    ;;
esac

echo "{
  \"trigger\": \"$TRIGGER\",
  \"context\": \"$CONTEXT\",
  \"priority\": \"$PRIORITY\",
  \"worker_dispatched\": true
}"
```

---

## Background Workers

### Worker Configuration

```yaml
# workers/config.yaml
# Background worker configuration for RustOps

workers:
  ultralearn:
    enabled: true
    schedule: "0 */6 * * *"  # Every 6 hours
    timeout: 3600  # 1 hour
    priority: normal
    resources:
      memory: 2048MB
      cpu: 1000m
    triggers:
      - after_critical_incident
      - after_new_pattern

  optimize:
    enabled: true
    schedule: "0 */4 * * *"  # Every 4 hours
    timeout: 1800  # 30 minutes
    priority: high
    resources:
      memory: 1024MB
      cpu: 500m
    triggers:
      - after_remediation
      - after_performance_degradation

  consolidate:
    enabled: true
    schedule: "0 2 * * *"  # Daily at 2 AM
    timeout: 600  # 10 minutes
    priority: low
    resources:
      memory: 512MB
      cpu: 200m
    triggers:
      - periodic

  audit:
    enabled: true
    schedule: "0 */8 * * *"  # Every 8 hours
    timeout: 1200  # 20 minutes
    priority: critical
    resources:
      memory: 1024MB
      cpu: 500m
    triggers:
      - after_security_event
      - after_unauthorized_remediation

  testgaps:
    enabled: true
    schedule: "0 3 * * 0"  # Weekly Sunday 3 AM
    timeout: 1800  # 30 minutes
    priority: normal
    resources:
      memory: 1024MB
      cpu: 500m
    triggers:
      - after_major_code_change

  map:
    enabled: true
    schedule: "0 1 * * *"  # Daily at 1 AM
    timeout: 300  # 5 minutes
    priority: normal
    resources:
      memory: 256MB
      cpu: 100m
    triggers:
      - after_service_change

  deepdive:
    enabled: true
    schedule: "manual"
    timeout: 3600  # 1 hour
    priority: normal
    resources:
      memory: 2048MB
      cpu: 1000m
    triggers:
      - on_demand
      - after_complex_incident

  document:
    enabled: true
    schedule: "0 4 * * *"  # Daily at 4 AM
    timeout: 600  # 10 minutes
    priority: low
    resources:
      memory: 512MB
      cpu: 200m
    triggers:
      - after_api_change
      - after_new_pattern

  preload:
    enabled: true
    schedule: "@startup"  # On startup
    timeout: 300  # 5 minutes
    priority: high
    resources:
      memory: 1024MB
      cpu: 500m
    triggers:
      - on_startup

  predict:
    enabled: true
    schedule: "0 * * * *"  # Every hour
    timeout: 300  # 5 minutes
    priority: normal
    resources:
      memory: 512MB
      cpu: 300m
    triggers:
      - periodic
      - after_capacity_change

  refactor:
    enabled: false  # Disabled by default
    schedule: "manual"
    timeout: 1800  # 30 minutes
    priority: low
    resources:
      memory: 1024MB
      cpu: 500m
    triggers:
      - on_demand

  benchmark:
    enabled: true
    schedule: "0 3 * * *"  # Daily at 3 AM
    timeout: 600  # 10 minutes
    priority: low
    resources:
      memory: 1024MB
      cpu: 500m
    triggers:
      - after_performance_change
      - weekly
```

### Worker Implementations

#### Ultralearn Worker

```rust
/// Ultralearn worker for deep knowledge acquisition
pub struct UltralearnWorker {
    memory: UnifiedMemoryService,
    sona: SONAIntegration,
}

impl UltralearnWorker {
    /// Run ultralearn cycle
    pub async fn run(&mut self, context: &str) -> Result<UltralearnReport> {
        let mut report = UltralearnReport::default();

        // 1. Extract recent incidents
        let recent_incidents = self
            .memory
            .query(MemoryQuery {
                query_type: QueryType::Structured,
                namespace: Some(MemoryNamespace::Incidents),
                content: None,
                filters: vec![QueryFilter::Recent(Duration::days(7))],
                limit: 100,
                threshold: None,
            })
            .await?;

        report.incidents_analyzed = recent_incidents.len();

        // 2. Extract patterns from incidents
        let extractor = PatternExtractor::new(0.8, 3);
        let patterns = extractor.extract_from_incidents(recent_incidents).await?;

        report.patterns_extracted = patterns.len();

        // 3. Validate patterns on staging
        let validated = self.validate_patterns_staging(&patterns).await?;

        report.patterns_validated = validated.len();

        // 4. Store validated patterns
        for pattern in validated {
            self.memory
                .store(MemoryEntry {
                    id: pattern.id.clone(),
                    namespace: MemoryNamespace::Patterns,
                    entry_type: MemoryEntryType::Pattern,
                    content: serde_json::to_string(&pattern)?,
                    embedding: None,
                    metadata: MemoryMetadata {
                        service: None,
                        environment: None,
                        severity: None,
                        related_incidents: pattern.provenance.clone(),
                        tags: vec!["ultralearn-extracted".to_string()],
                        source: DataSource::System {
                            component: "ultralearn-worker".to_string(),
                        },
                    },
                    temporal: TemporalData {
                        created_at: Utc::now(),
                        last_accessed: Utc::now(),
                        access_count: 1,
                        ttl: None,
                        decay_rate: 0.01,
                    },
                    learning: None,
                })
                .await?;
        }

        // 5. Trigger SONA consolidation
        self.sona.consolidate_recent_trajectories(7).await?;

        report.consolidation_complete = true;

        Ok(report)
    }
}

#[derive(Debug, Default)]
pub struct UltralearnReport {
    pub incidents_analyzed: usize,
    pub patterns_extracted: usize,
    pub patterns_validated: usize,
    pub consolidation_complete: bool,
}
```

#### Optimize Worker

```rust
/// Optimize worker for performance optimization
pub struct OptimizeWorker {
    memory: UnifiedMemoryService,
    performance_monitor: PerformanceMonitor,
}

impl OptimizeWorker {
    /// Run optimization cycle
    pub async fn run(&mut self, context: &str) -> Result<OptimizeReport> {
        let mut report = OptimizeReport::default();

        // 1. Find performance bottlenecks
        let bottlenecks = self.performance_monitor.detect_bottlenecks().await?;

        report.bottlenecks_detected = bottlenecks.len();

        // 2. Generate optimization suggestions
        let suggestions = self.generate_optimizations(&bottlenecks).await?;

        report.optimizations_suggested = suggestions.len();

        // 3. Apply safe optimizations automatically
        for suggestion in &suggestions {
            if suggestion.risk_level == RiskLevel::Low {
                self.apply_optimization(suggestion).await?;
                report.optimizations_applied += 1;
            }
        }

        // 4. Store high-risk suggestions for review
        let high_risk: Vec<_> = suggestions
            .into_iter()
            .filter(|s| s.risk_level != RiskLevel::Low)
            .collect();

        for suggestion in high_risk {
            self.memory
                .store(MemoryEntry {
                    id: ulid::new().to_string(),
                    namespace: MemoryNamespace::Feedback,
                    entry_type: MemoryEntryType::Feedback,
                    content: serde_json::to_string(&suggestion)?,
                    embedding: None,
                    metadata: MemoryMetadata {
                        service: suggestion.service.clone(),
                        environment: None,
                        severity: Some(SeverityLevel::Medium),
                        related_incidents: vec![],
                        tags: vec!["optimization-suggestion".to_string()],
                        source: DataSource::System {
                            component: "optimize-worker".to_string(),
                        },
                    },
                    temporal: TemporalData {
                        created_at: Utc::now(),
                        last_accessed: Utc::now(),
                        access_count: 1,
                        ttl: Some(30 * 24 * 3600),
                        decay_rate: 0.05,
                    },
                    learning: None,
                })
                .await?;
        }

        Ok(report)
    }
}
```

#### Audit Worker

```rust
/// Audit worker for security analysis
pub struct AuditWorker {
    memory: UnifiedMemoryService,
    security_analyzer: SecurityAnalyzer,
}

impl AuditWorker {
    /// Run security audit cycle
    pub async fn run(&mut self, context: &str) -> Result<AuditReport> {
        let mut report = AuditReport::default();

        // 1. Get recent remediation actions
        let recent_actions = self
            .memory
            .query(MemoryQuery {
                query_type: QueryType::Structured,
                namespace: Some(MemoryNamespace::ConfigChanges),
                content: None,
                filters: vec![QueryFilter::Recent(Duration::hours(24))],
                limit: 100,
                threshold: None,
            })
            .await?;

        // 2. Analyze each action for security risks
        for action in &recent_actions {
            let risks = self.security_analyzer.analyze_action(action).await?;

            if !risks.is_empty() {
                report.security_issues_found += risks.len();

                // Store security findings
                for risk in &risks {
                    self.memory
                        .store(MemoryEntry {
                            id: ulid::new().to_string(),
                            namespace: MemoryNamespace::Feedback,
                            entry_type: MemoryEntryType::Feedback,
                            content: serde_json::to_string(&risk)?,
                            embedding: None,
                            metadata: MemoryMetadata {
                                service: None,
                                environment: None,
                                severity: Some(risk.severity),
                                related_incidents: vec![],
                                tags: vec!["security-issue".to_string()],
                                source: DataSource::System {
                                    component: "audit-worker".to_string(),
                                },
                            },
                            temporal: TemporalData {
                                created_at: Utc::now(),
                                last_accessed: Utc::now(),
                                access_count: 1,
                                ttl: Some(180 * 24 * 3600),
                                decay_rate: 0.02,
                            },
                            learning: None,
                        })
                        .await?;
                }
            }
        }

        Ok(report)
    }
}
```

#### Testgaps Worker

```rust
/// Testgaps worker for test coverage analysis
pub struct TestgapsWorker {
    memory: UnifiedMemoryService,
    code_analyzer: CodeAnalyzer,
}

impl TestgapsWorker {
    /// Run test gap analysis cycle
    pub async fn run(&mut self, context: &str) -> Result<TestgapsReport> {
        let mut report = TestgapsReport::default();

        // 1. Get recent incident edge cases
        let edge_cases = self
            .memory
            .query(MemoryQuery {
                query_type: QueryType::Semantic,
                namespace: Some(MemoryNamespace::Incidents),
                content: Some("edge case unusual rare unexpected".to_string()),
                filters: vec![QueryFilter::Recent(Duration::days(30))],
                limit: 50,
                threshold: Some(0.7),
            })
            .await?;

        // 2. Analyze test coverage for edge cases
        let gaps = self.code_analyzer.find_test_gaps(&edge_cases).await?;

        report.test_gaps_found = gaps.len();

        // 3. Generate test suggestions
        for gap in &gaps {
            self.memory
                .store(MemoryEntry {
                    id: ulid::new().to_string(),
                    namespace: MemoryNamespace::Feedback,
                    entry_type: MemoryEntryType::Feedback,
                    content: serde_json::to_string(&gap)?,
                    embedding: None,
                    metadata: MemoryMetadata {
                        service: gap.service.clone(),
                        environment: None,
                        severity: Some(SeverityLevel::Low),
                        related_incidents: vec![gap.incident_id.clone()],
                        tags: vec!["test-gap".to_string()],
                        source: DataSource::System {
                            component: "testgaps-worker".to_string(),
                        },
                    },
                    temporal: TemporalData {
                        created_at: Utc::now(),
                        last_accessed: Utc::now(),
                        access_count: 1,
                        ttl: Some(90 * 24 * 3600),
                        decay_rate: 0.03,
                    },
                    learning: None,
                })
                .await?;
        }

        Ok(report)
    }
}
```

---

## Integration Examples

### Complete Incident Resolution Flow

```rust
/// Complete incident resolution with hooks and workers
pub async fn resolve_incident_with_hooks(
    incident: Incident,
    memory: UnifiedMemoryService,
) -> Result<ResolutionResult> {
    let result = ResolutionResult::default();

    // 1. Pre-task: Route to optimal agent
    let pre_task = hooks::pre_task(
        incident.id.clone(),
        incident.incident_type.clone(),
        incident.service.clone(),
        incident.environment.clone(),
        incident.severity.clone(),
    )
    .await?;

    result.agent_used = pre_task.routed_to;
    result.sona_mode = pre_task.sona_mode;

    // 2. Resolve incident using agent
    let resolution = resolve_with_agent(incident.clone(), &pre_task).await?;
    result.resolution = resolution.clone();

    // 3. Extract and store pattern if successful
    if resolution.success {
        let quality = calculate_quality(&resolution);

        // Post-task: Store pattern
        let post_task = hooks::post_task(
            incident.id.clone(),
            true,
            quality,
            resolution.duration_seconds,
        )
        .await?;

        result.pattern_stored = post_task.pattern_stored;

        // Dispatch workers based on result
        if incident.severity == SeverityLevel::Critical {
            hooks::worker_dispatch("ultralearn", &incident.id, "high").await?;
        }

        hooks::worker_dispatch("optimize", &incident.service, "normal").await?;
    } else {
        // Log failure for learning
        hooks::post_task(incident.id.clone(), false, 0.0, resolution.duration_seconds).await?;
    }

    // 4. If config changes were made, track them
    for change in &resolution.config_changes {
        hooks::post_edit(
            change.file_path.clone(),
            "update",
            change.user.clone(),
        )
        .await?;
    }

    Ok(result)
}
```

---

## Worker Status Monitoring

```rust
/// Worker status monitoring
pub struct WorkerMonitor {
    workers: HashMap<String, WorkerStatus>,
}

impl WorkerMonitor {
    /// Get status of all workers
    pub async fn get_all_status(&self) -> Vec<WorkerStatus> {
        self.workers.values().cloned().collect()
    }

    /// Get specific worker status
    pub async fn get_worker_status(&self, worker_name: &str) -> Option<WorkerStatus> {
        self.workers.get(worker_name).cloned()
    }

    /// Trigger worker manually
    pub async fn trigger_worker(&mut self, worker_name: &str, context: &str) -> Result<WorkerResult> {
        let worker = self.workers.get_mut(worker_name)
            .ok_or_else(|| anyhow!("Worker not found: {}", worker_name))?;

        worker.last_triggered = Utc::now();
        worker.status = WorkerStatusState::Running;

        // Execute worker (implementation depends on worker type)
        let result = match worker_name {
            "ultralearn" => self.run_ultralearn(context).await?,
            "optimize" => self.run_optimize(context).await?,
            "consolidate" => self.run_consolidate(context).await?,
            "audit" => self.run_audit(context).await?,
            "testgaps" => self.run_testgaps(context).await?,
            _ => bail!("Unknown worker: {}", worker_name),
        };

        worker.last_completed = Utc::now();
        worker.status = WorkerStatusState::Idle;
        worker.total_runs += 1;

        Ok(result)
    }
}

#[derive(Debug, Clone)]
pub struct WorkerStatus {
    pub name: String,
    pub status: WorkerStatusState,
    pub enabled: bool,
    pub schedule: String,
    pub last_triggered: Option<DateTime<Utc>>,
    pub last_completed: Option<DateTime<Utc>>,
    pub total_runs: u64,
    pub average_duration_seconds: f64,
}

#[derive(Debug, Clone)]
pub enum WorkerStatusState {
    Idle,
    Running,
    Failed,
    Disabled,
}
```

---

## Summary

| Hook | Purpose | Trigger | Storage |
|------|---------|---------|---------|
| pre-task | Agent routing with SONA | Incident start | incidents |
| post-task | Pattern storage | Incident resolution | patterns |
| post-edit | Config tracking | Config changes | config-changes |
| worker-dispatch | Background jobs | Various events | - |

| Worker | Frequency | Duration | Priority | Output |
|--------|-----------|----------|----------|--------|
| ultralearn | Every 6h | 1h | normal | New patterns |
| optimize | Every 4h | 30m | high | Performance improvements |
| consolidate | Daily | 10m | low | Memory cleanup |
| audit | Every 8h | 20m | critical | Security findings |
| testgaps | Weekly | 30m | normal | Test suggestions |
| map | Daily | 5m | normal | Service topology |
| deepdive | Manual | 1h | normal | Deep analysis |
| document | Daily | 10m | low | Documentation |
| preload | Startup | 5m | high | Cached data |
| predict | Hourly | 5m | normal | Predictions |

---

**Document Version**: 1.0
**Last Updated**: 2026-01-18
**Author**: Integration Team
