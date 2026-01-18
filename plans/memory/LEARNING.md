# Learning Capabilities Specification

**SONA Integration: Self-Optimizing Neural Architecture**

## Overview

RustOps implements a comprehensive learning system that continuously improves from operational experience using Claude Flow's SONA (Self-Optimizing Neural Architecture) with LoRA fine-tuning, EWC++ consolidation, and trajectory tracking.

---

## Learning Pipeline

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        RustOps Learning Pipeline                             │
├─────────────────────────────────────────────────────────────────────────────┤
│                                                                              │
│  ┌────────────────┐    ┌────────────────┐    ┌────────────────┐            │
│  │  INCIDENT      │    │  ANOMALY       │    │  FEEDBACK      │            │
│  │  RESOLUTION    │    │  DETECTION     │    │  OVERRIDE      │            │
│  └────────┬───────┘    └────────┬───────┘    └────────┬───────┘            │
│           │                     │                     │                     │
│           └─────────────────────┼─────────────────────┘                     │
│                                 ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    1. TRAJECTORY TRACKING                            │  │
│  │  - Record each action taken                                         │  │
│  │  - Track state changes                                               │  │
│  │  - Measure quality (0-1)                                             │  │
│  │  - Store temporal sequence                                           │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    2. VERDICT JUDGMENT                               │  │
│  │  - Success: Resolution achieved, user satisfied                      │  │
│  │  - Partial: Partial resolution, mixed feedback                       │  │
│  │  - Failure: Resolution failed, negative feedback                     │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    3. REWARD CALCULATION                             │  │
│  │  - Time to resolve (faster = better)                                 │  │
│  │  - Success rate (higher = better)                                    │  │
│  │  - User satisfaction (feedback score)                                │  │
│  │  - Resource efficiency (minimal actions)                              │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    4. LORA FINE-TUNING                               │  │
│  │  - Adapt model to domain-specific patterns                           │  │
│  │  - Mode-specific optimization (real-time/balanced/research)          │  │
│  │  - <0.05ms adaptation latency                                        │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    5. MEMORY DISTILLATION                            │  │
│  │  - Extract key learnings from trajectory                             │  │
│  │  - Create reusable patterns                                          │  │
│  │  - Store in AgentDB with embeddings                                  │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    6. EWC++ CONSOLIDATION                            │  │
│  │  - Calculate Fisher information matrix                               │  │
│  │  - Preserve important parameters                                      │  │
│  │  - Prevent catastrophic forgetting                                    │  │
│  └───────────────────────────────┬───────────────────────────────────────┘  │
│                                  ▼                                           │
│  ┌───────────────────────────────────────────────────────────────────────┐  │
│  │                    7. PATTERN VALIDATION                             │  │
│  │  - Test on staging environment                                       │  │
│  │  - Validate effectiveness                                            │  │
│  │  - Mark as validated/tested/extracted                                 │  │
│  └───────────────────────────────────────────────────────────────────────┘  │
│                                                                              │
└─────────────────────────────────────────────────────────────────────────────┘
```

---

## 1. Pattern Recognition

### Extracting Remediation Patterns

```rust
/// Pattern extraction from incident resolution
pub struct PatternExtractor {
    /// Minimum success rate for pattern extraction
    min_success_rate: f32,

    /// Minimum occurrences to consider as pattern
    min_occurrences: u64,
}

impl PatternExtractor {
    /// Extract patterns from resolved incidents
    pub async fn extract_from_incidents(
        &self,
        incidents: Vec<Incident>,
    ) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();

        // Group incidents by similarity
        let groups = self.group_by_similarity(&incidents, 0.85)?;

        for group in groups {
            // Only extract from successful resolutions
            let success_rate = group.success_rate();
            if success_rate < self.min_success_rate {
                continue;
            }

            // Extract common resolution steps
            let common_steps = self.extract_common_steps(&group)?;

            // Extract trigger conditions
            let triggers = self.extract_triggers(&group)?;

            // Create pattern
            let pattern = Pattern {
                id: ulid::new().to_string(),
                name: self.generate_pattern_name(&group),
                description: self.generate_description(&group),
                triggers,
                actions: common_steps,
                expected_outcome: group.expected_outcome(),
                success_rate,
                usage_count: group.len() as u64,
                last_used: Utc::now(),
                applicability: Applicability {
                    services: group.unique_services(),
                    environments: group.unique_environments(),
                },
                risk_level: self.assess_risk(&group),
                estimated_duration_seconds: group.median_duration(),
                provenance: group.iter().map(|i| i.id.clone()).collect(),
                validation_status: ValidationStatus::Extracted,
                recommended_sona_mode: SonamMode::Balanced,
            };

            patterns.push(pattern);
        }

        Ok(patterns)
    }

    /// Group incidents by semantic similarity
    fn group_by_similarity(
        &self,
        incidents: &[Incident],
        threshold: f32,
    ) -> Result<Vec<Vec<Incident>>> {
        let mut groups = Vec::new();
        let mut assigned = HashSet::new();

        for (i, incident) in incidents.iter().enumerate() {
            if assigned.contains(&i) {
                continue;
            }

            let mut group = vec![incident.clone()];
            assigned.insert(i);

            // Find similar incidents
            for (j, other) in incidents.iter().enumerate() {
                if i == j || assigned.contains(&j) {
                    continue;
                }

                let similarity = self.compute_similarity(incident, other)?;
                if similarity >= threshold {
                    group.push(other.clone());
                    assigned.insert(j);
                }
            }

            // Only keep groups with minimum occurrences
            if group.len() as u64 >= self.min_occurrences {
                groups.push(group);
            }
        }

        Ok(groups)
    }

    /// Extract common resolution steps
    fn extract_common_steps(&self, incidents: &[Incident]) -> Result<Vec<Action>> {
        // Collect all resolution steps
        let all_steps: Vec<&ResolutionStep> = incidents
            .iter()
            .flat_map(|i| i.resolution_steps.iter())
            .collect();

        // Find common sequences using sequence mining
        let common_sequences = self.mine_common_sequences(&all_steps, 0.7)?;

        // Convert to actions
        let actions = common_sequences
            .into_iter()
            .map(|seq| Action {
                order: seq.order,
                action_type: seq.action_type,
                parameters: seq.parameters,
                timeout_seconds: seq.timeout,
                rollback: seq.rollback,
                validation_checks: seq.validation,
            })
            .collect();

        Ok(actions)
    }

    /// Mine common action sequences
    fn mine_common_sequences(
        &self,
        steps: &[&ResolutionStep],
        min_support: f32,
    ) -> Result<Vec<ActionSequence>> {
        // Use PrefixSpan algorithm for sequence mining
        let sequences = self.prefixspan_mining(steps, min_support)?;
        Ok(sequences)
    }
}
```

### Pattern Learning from Feedback

```rust
/// Update pattern based on feedback
pub async fn update_pattern_from_feedback(
    pattern: &mut Pattern,
    feedback: &Feedback,
) -> Result<PatternUpdate> {
    match feedback.feedback {
        FeedbackValue::Boolean(true) => {
            // Positive feedback - increase success rate
            pattern.success_rate = (pattern.success_rate * 0.9) + (1.0 * 0.1);

            // Mark as validated if enough positive feedback
            if pattern.success_rate > 0.9 && pattern.usage_count > 10 {
                pattern.validation_status = ValidationStatus::Validated;
            }
        }
        FeedbackValue::Boolean(false) => {
            // Negative feedback - decrease success rate
            pattern.success_rate = pattern.success_rate * 0.8;

            // Mark as invalid if too many failures
            if pattern.success_rate < 0.5 {
                pattern.validation_status = ValidationStatus::Invalid;
            }
        }
        FeedbackValue::Rating(rating) => {
            // Update success rate based on rating (1-5)
            let normalized = (rating - 1) as f32 / 4.0; // 0-1
            pattern.success_rate = (pattern.success_rate * 0.8) + (normalized * 0.2);
        }
        FeedbackValue::AlternativeAction(ref action) => {
            // Suggest alternative action
            pattern.actions.push(action.clone());
        }
        _ => {}
    }

    pattern.usage_count += 1;
    pattern.last_used = Utc::now();

    Ok(PatternUpdate {
        pattern_id: pattern.id.clone(),
        old_success_rate: pattern.success_rate,
        new_success_rate: pattern.success_rate,
        validation_status: pattern.validation_status.clone(),
    })
}
```

---

## 2. Anomaly Learning

### False Positive/Negative Learning

```rust
/// Anomaly detector with online learning
pub struct AdaptiveAnomalyDetector {
    /// Base detection model
    base_model: AnomalyModel,

    /// SONA integration
    sona: SONAIntegration,

    /// False positive patterns
    fp_patterns: Vec<FalsePositivePattern>,

    /// Thresholds per metric
    adaptive_thresholds: HashMap<String, AdaptiveThreshold>,
}

impl AdaptiveAnomalyDetector {
    /// Detect anomaly with learning
    pub async fn detect_with_learning(
        &mut self,
        metric_value: &MetricValue,
    ) -> Result<AnomalyResult> {
        // Check against false positive patterns first
        if self.is_likely_false_positive(metric_value)? {
            return Ok(AnomalyResult {
                is_anomaly: false,
                confidence: 0.9,
                reason: "Matches known false positive pattern".to_string(),
                label: AnomalyLabel::FalsePositive,
            });
        }

        // Use adaptive threshold
        let threshold = self
            .adaptive_thresholds
            .get(&metric_value.name)
            .map(|t| t.current_threshold)
            .unwrap_or(metric_value.baseline_threshold);

        // Detect anomaly
        let is_anomaly = self.base_model.detect(metric_value, threshold)?;

        // If anomaly, check against patterns
        if is_anomaly {
            // Similar past anomalies?
            let past_anomalies = self.find_similar_anomalies(metric_value).await?;

            if let Some(true_positive) = self.check_true_positive_pattern(&past_anomalies) {
                return Ok(true_positive);
            }
        }

        Ok(AnomalyResult {
            is_anomaly,
            confidence: self.base_model.confidence(),
            reason: String::new(),
            label: AnomalyLabel::Unknown,
        })
    }

    /// Learn from labeled anomaly
    pub async fn learn_from_label(
        &mut self,
        anomaly: &AnomalyRecord,
        label: AnomalyLabel,
    ) -> Result<()> {
        match label {
            AnomalyLabel::FalsePositive => {
                // Extract false positive pattern
                let pattern = FalsePositivePattern {
                    id: ulid::new().to_string(),
                    metric_name: anomaly.target.metric_name(),
                    conditions: self.extract_fp_conditions(anomaly),
                    time_patterns: self.extract_time_patterns(anomaly),
                    context_patterns: self.extract_context_patterns(anomaly),
                };

                self.fp_patterns.push(pattern);

                // Adjust threshold upward
                if let Some(threshold) = self.adaptive_thresholds.get_mut(&pattern.metric_name) {
                    threshold.false_positive_count += 1;
                    threshold.current_threshold *= 1.1; // Increase by 10%
                }
            }
            AnomalyLabel::TruePositive => {
                // Adjust threshold downward if appropriate
                if let Some(threshold) = self.adaptive_thresholds.get_mut(&anomaly.target.metric_name()) {
                    threshold.true_positive_count += 1;

                    // If we're missing too many, lower threshold
                    let precision =
                        threshold.true_positive_count as f32 / (threshold.true_positive_count as f32 + threshold.false_positive_count as f32);

                    if precision < 0.7 {
                        threshold.current_threshold *= 0.95; // Decrease by 5%
                    }
                }
            }
            AnomalyLabel::FalseNegative => {
                // We missed this - lower threshold significantly
                if let Some(threshold) = self.adaptive_thresholds.get_mut(&anomaly.target.metric_name()) {
                    threshold.false_negative_count += 1;
                    threshold.current_threshold *= 0.8; // Decrease by 20%
                }
            }
            _ => {}
        }

        // Trigger SONA learning
        self.sona
            .train_trajectory(&ResolutionTrajectory {
                id: ulid::new().to_string(),
                incident_id: anomaly.related_incident.clone().unwrap_or_default(),
                steps: vec![],
                user_feedback: Some(match label {
                    AnomalyLabel::TruePositive => 1.0,
                    AnomalyLabel::FalsePositive => -1.0,
                    AnomalyLabel::FalseNegative => -0.5,
                    _ => 0.0,
                }),
            })
            .await?;

        Ok(())
    }
}

#[derive(Debug, Clone)]
struct AdaptiveThreshold {
    metric_name: String,
    initial_threshold: f32,
    current_threshold: f32,
    true_positive_count: u64,
    false_positive_count: u64,
    false_negative_count: u64,
}
```

---

## 3. Topology Learning

### Service Dependency Discovery

```rust
/// Service topology learner
pub struct TopologyLearner {
    /// Current topology graph
    graph: DependencyGraph,

    /// Historical topologies
    history: Vec<TopologySnapshot>,
}

impl TopologyLearner {
    /// Learn from communication patterns
    pub async fn learn_from_communication(
        &mut self,
        communication: &CommunicationEvent,
    ) -> Result<TopologyChange> {
        let mut changes = TopologyChange::default();

        // Check if this is a new dependency
        if !self.graph.has_edge(&communication.from, &communication.to) {
            // New edge discovered
            self.graph.add_edge(DependencyEdge {
                id: ulid::new().to_string(),
                from: communication.from.clone(),
                to: communication.to.clone(),
                dependency_type: self.infer_dependency_type(communication),
                protocol: communication.protocol.clone(),
                call_rate: None,
                error_rate: None,
                latency: LatencyMetrics::default(),
                criticality: EdgeCriticality::Unknown,
            });

            changes.edges_added.push(format!("{}->{}", communication.from, communication.to));
        }

        // Update edge metrics
        if let Some(edge) = self.graph.get_edge_mut(&communication.from, &communication.to) {
            edge.call_rate = Some(communication.call_rate);
            edge.error_rate = Some(communication.error_rate);
            edge.latency = communication.latency.clone();

            // Update criticality based on error rate and call rate
            edge.criticality = self.assess_criticality(edge);
        }

        Ok(changes)
    }

    /// Detect topology anomalies
    pub async fn detect_anomalies(&self) -> Result<Vec<TopologyAnomaly>> {
        let mut anomalies = Vec::new();

        // 1. Missing dependencies (service not responding)
        for edge in self.graph.edges() {
            if edge.error_rate.unwrap_or(0.0) > 0.5 {
                anomalies.push(TopologyAnomaly {
                    anomaly_type: TopologyAnomalyType::DependencyFailure {
                        dependency: format!("{}->{}", edge.from, edge.to),
                        error_rate: edge.error_rate.unwrap(),
                    },
                    severity: SeverityLevel::High,
                });
            }
        }

        // 2. Unexpected new dependencies
        let recent_snapshot = self.history.last();
        if let Some(recent) = recent_snapshot {
            let current_edges: HashSet<_> = self.graph.edge_ids().collect();
            let recent_edges: HashSet<_> = recent.edge_ids().collect();

            let new_edges: Vec<_> = current_edges.difference(&recent_edges).cloned().collect();

            for edge in new_edges {
                anomalies.push(TopologyAnomaly {
                    anomaly_type: TopologyAnomalyType::UnexpectedDependency {
                        dependency: edge,
                    },
                    severity: SeverityLevel::Medium,
                });
            }
        }

        // 3. Critical path changes
        let current_critical_path = self.graph.find_critical_path();
        if let Some(recent) = recent_snapshot {
            let recent_critical_path = recent.find_critical_path();

            if current_critical_path != recent_critical_path {
                anomalies.push(TopologyAnomaly {
                    anomaly_type: TopologyAnomalyType::CriticalPathChanged {
                        old_path: recent_critical_path,
                        new_path: current_critical_path,
                    },
                    severity: SeverityLevel::Medium,
                });
            }
        }

        Ok(anomalies)
    }
}
```

---

## 4. Runbook Extraction

### Extract Runbooks from Incidents

```rust
/// Runbook extractor from incident resolutions
pub struct RunbookExtractor {
    /// NLP model for natural language understanding
    nlp_model: NLPModel,
}

impl RunbookExtractor {
    /// Extract runbook from successful incident resolution
    pub async fn extract_from_incident(
        &self,
        incident: &Incident,
    ) -> Result<Runbook> {
        // Only extract from successful resolutions
        if incident.status != IncidentStatus::Resolved {
            bail!("Can only extract from resolved incidents");
        }

        // Extract triggers from incident description
        let triggers = self.extract_triggers(incident).await?;

        // Extract steps from resolution steps
        let steps = self.extract_steps(incident).await?;

        // Generate prerequisites
        let prerequisites = self.infer_prerequisites(&steps).await?;

        // Generate rollback steps
        let rollback_steps = self.generate_rollback(&steps).await?;

        // Estimate duration
        let estimated_duration = incident.resolved_at.unwrap() - incident.detected_at;

        Ok(Runbook {
            id: ulid::new().to_string(),
            name: self.generate_name(incident),
            description: incident.description.clone(),
            triggers,
            steps,
            prerequisites,
            expected_outcomes: vec!["Issue resolved".to_string()],
            rollback_steps,
            estimated_duration_seconds: estimated_duration.num_seconds() as u64,
            risk_level: self.assess_risk(&steps),
            validation_status: ValidationStatus::Extracted,
            last_validated: None,
            success_rate: 1.0, // From successful resolution
            usage_count: 0,
            source: RunbookSource::Extracted {
                incident_id: incident.id.clone(),
            },
            version: "1.0.0".to_string(),
            recommended_sona_mode: SonamMode::Balanced,
        })
    }

    /// Extract trigger conditions from incident
    async fn extract_triggers(&self, incident: &Incident) -> Result<Vec<TriggerCondition>> {
        let mut triggers = Vec::new();

        // Parse incident description for symptom mentions
        let symptoms = self.nlp_model.extract_symptoms(&incident.description).await?;

        for symptom in symptoms {
            let trigger = match symptom.type_ {
                SymptomType::MetricSpike => TriggerCondition {
                    condition_type: TriggerType::MetricThreshold {
                        metric_name: symptom.metric_name.unwrap(),
                        operator: ComparisonOp::GreaterThan,
                        value: symptom.value.unwrap(),
                    },
                    expression: format!("{} > {}", symptom.metric_name.unwrap(), symptom.value.unwrap()),
                    threshold: Some(serde_json::json!(symptom.value.unwrap())),
                    time_window_seconds: 300, // 5 minutes
                },
                SymptomType::ErrorRate => TriggerCondition {
                    condition_type: TriggerType::ErrorRateSike {
                        baseline: symptom.baseline.unwrap(),
                        threshold: symptom.value.unwrap(),
                        window: 300,
                    },
                    expression: format!("error_rate > {}", symptom.value.unwrap()),
                    threshold: Some(serde_json::json!(symptom.value.unwrap())),
                    time_window_seconds: 300,
                },
                SymptomType::LogPattern => TriggerCondition {
                    condition_type: TriggerType::LogPattern {
                        pattern: symptom.pattern.unwrap(),
                        min_occurrences: 10,
                        time_window: 300,
                    },
                    expression: format!("logs match pattern: {}", symptom.pattern.unwrap()),
                    threshold: Some(serde_json::json!(10)),
                    time_window_seconds: 300,
                },
                _ => continue,
            };

            triggers.push(trigger);
        }

        Ok(triggers)
    }

    /// Extract structured steps from resolution
    async fn extract_steps(&self, incident: &Incident) -> Result<Vec<RunbookStep>> {
        let mut steps = Vec::new();

        for (i, resolution_step) in incident.resolution_steps.iter().enumerate() {
            let step = RunbookStep {
                step_number: i as u32 + 1,
                description: resolution_step.description.clone(),
                action: resolution_step.action.clone(),
                validation_checks: resolution_step.validation_checks.clone(),
                timeout_seconds: resolution_step.timeout_seconds,
                parallelizable: self.check_parallelizable(resolution_step),
                depends_on_steps: vec![i as u32], // Sequential dependency
            };

            steps.push(step);
        }

        Ok(steps)
    }
}
```

---

## 5. Feedback Integration

### Learning from Engineer Overrides

```rust
/// Feedback integration system
pub struct FeedbackIntegration {
    /// Memory service
    memory: UnifiedMemoryService,

    /// SONA integration
    sona: SONAIntegration,
}

impl FeedbackIntegration {
    /// Process feedback and learn from it
    pub async fn process_feedback(&mut self, feedback: Feedback) -> Result<FeedbackResult> {
        let mut result = FeedbackResult::default();

        // Store feedback in memory
        self.memory
            .store(MemoryEntry {
                id: feedback.id.clone(),
                namespace: MemoryNamespace::Feedback,
                entry_type: MemoryEntryType::Feedback,
                content: serde_json::to_string(&feedback)?,
                embedding: None,
                metadata: MemoryMetadata {
                    service: feedback.context.service.clone(),
                    environment: feedback.context.environment.clone(),
                    severity: None,
                    related_incidents: feedback
                        .context
                        .incident_id
                        .clone()
                        .map(|id| vec![id])
                        .unwrap_or_default(),
                    tags: vec![],
                    source: DataSource::Human {
                        user_id: feedback.source.user_id().to_string(),
                    },
                },
                temporal: TemporalData {
                    created_at: feedback.timestamp,
                    last_accessed: Utc::now(),
                    access_count: 1,
                    ttl: Some(180 * 24 * 3600), // 180 days
                    decay_rate: 0.03,
                },
                learning: None,
            })
            .await?;

        // Act on feedback based on type
        match feedback.feedback_type {
            FeedbackType::Override => {
                // Learn override pattern
                let learned = self.learn_override(&feedback).await?;
                result.learned_pattern = Some(learned);
            }
            FeedbackType::Confirm => {
                // Reinforce pattern
                self.reinforce_pattern(&feedback).await?;
                result.pattern_reinforced = true;
            }
            FeedbackType::Reject => {
                // Suppress pattern
                self.suppress_pattern(&feedback).await?;
                result.pattern_suppressed = true;
            }
            FeedbackType::Suggest => {
                // Extract suggestion
                let suggestion = self.extract_suggestion(&feedback).await?;
                result.suggestion_extracted = Some(suggestion);
            }
            FeedbackType::ReportIssue => {
                // Create issue for review
                self.create_issue(&feedback).await?;
                result.issue_created = true;
            }
        }

        // Trigger SONA learning trajectory
        if let Some(incident_id) = &feedback.context.incident_id {
            let trajectory = ResolutionTrajectory {
                id: ulid::new().to_string(),
                incident_id: incident_id.clone(),
                steps: vec![],
                user_feedback: Some(self.feedback_to_score(&feedback)),
            };

            self.sona.train_trajectory(&trajectory).await?;
        }

        Ok(result)
    }

    /// Learn from override feedback
    async fn learn_override(&mut self, feedback: &Feedback) -> Result<Pattern> {
        if let FeedbackValue::AlternativeAction(action) = &feedback.feedback {
            // Extract pattern from override action
            let pattern = Pattern {
                id: ulid::new().to_string(),
                name: format!("Override Pattern for {}", feedback.target.service()),
                description: format!(
                    "Pattern learned from override by {}",
                    feedback.source.user_id()
                ),
                triggers: vec![], // Will be populated from context
                actions: vec![action.clone()],
                expected_outcome: String::new(),
                success_rate: 0.8, // Start with moderate confidence
                usage_count: 1,
                last_used: Utc::now(),
                applicability: Applicability::default(),
                risk_level: RiskLevel::Medium,
                estimated_duration_seconds: 300,
                provenance: vec![feedback.context.incident_id.clone().unwrap_or_default()],
                validation_status: ValidationStatus::Extracted,
                recommended_sona_mode: SonamMode::Balanced,
            };

            // Store pattern
            self.memory
                .store(MemoryEntry {
                    id: pattern.id.clone(),
                    namespace: MemoryNamespace::Patterns,
                    entry_type: MemoryEntryType::Pattern,
                    content: serde_json::to_string(&pattern)?,
                    embedding: None,
                    metadata: MemoryMetadata {
                        service: feedback.context.service.clone(),
                        environment: feedback.context.environment.clone(),
                        severity: None,
                        related_incidents: feedback
                            .context
                            .incident_id
                            .clone()
                            .map(|id| vec![id])
                            .unwrap_or_default(),
                        tags: vec!["learned-from-override".to_string()],
                        source: DataSource::Human {
                            user_id: feedback.source.user_id().to_string(),
                        },
                    },
                    temporal: TemporalData {
                        created_at: Utc::now(),
                        last_accessed: Utc::now(),
                        access_count: 1,
                        ttl: None, // Persistent
                        decay_rate: 0.01,
                    },
                    learning: None,
                })
                .await?;

            Ok(pattern)
        } else {
            bail!("Not an override feedback")
        }
    }
}
```

---

## 6. SONA Integration Details

### Mode Selection Strategy

```rust
/// SONA mode selector
pub struct SonaModeSelector {
    /// Latency requirements
    latency_targets: HashMap<String, u64>, // operation -> target ms

    /// Complexity thresholds
    complexity_thresholds: ComplexityThresholds,
}

impl SonaModeSelector {
    /// Select appropriate SONA mode for operation
    pub fn select_mode(&self, operation: &Operation) -> SonamMode {
        // Real-time mode for low-latency operations
        if let Some(target) = self.latency_targets.get(&operation.name) {
            if *target < 10 {
                return SonamMode::RealTime;
            }
        }

        // Research mode for complex operations
        if operation.complexity() > self.complexity_thresholds.max_complexity {
            return SonamMode::Research;
        }

        // Edge mode for handling edge cases
        if operation.is_edge_case {
            return SonamMode::Edge;
        }

        // Batch mode for bulk operations
        if operation.is_batch {
            return SonamMode::Batch;
        }

        // Default to balanced mode
        SonamMode::Balanced
    }
}

/// SONA mode performance characteristics
pub struct SonaModeCharacteristics {
    pub mode: SonamMode,
    pub latency_ms: f64,
    pub accuracy: f32,
    pub cost_per_operation: f32,
    pub use_cases: Vec<String>,
}

impl SonaModeCharacteristics {
    pub fn all_modes() -> Vec<Self> {
        vec![
            SonaModeCharacteristics {
                mode: SonamMode::RealTime,
                latency_ms: 0.8,
                accuracy: 0.85,
                cost_per_operation: 0.0001,
                use_cases: vec![
                    "Real-time alerting".to_string(),
                    "Interactive decisions".to_string(),
                ],
            },
            SonaModeCharacteristics {
                mode: SonamMode::Balanced,
                latency_ms: 5.0,
                accuracy: 0.92,
                cost_per_operation: 0.0005,
                use_cases: vec![
                    "Incident investigation".to_string(),
                    "Pattern matching".to_string(),
                ],
            },
            SonaModeCharacteristics {
                mode: SonamMode::Research,
                latency_ms: 50.0,
                accuracy: 0.98,
                cost_per_operation: 0.005,
                use_cases: vec![
                    "Deep analysis".to_string(),
                    "Complex decisions".to_string(),
                ],
            },
            SonaModeCharacteristics {
                mode: SonamMode::Edge,
                latency_ms: 10.0,
                accuracy: 0.75,
                cost_per_operation: 0.001,
                use_cases: vec![
                    "Edge case handling".to_string(),
                    "Novel situations".to_string(),
                ],
            },
            SonaModeCharacteristics {
                mode: SonamMode::Batch,
                latency_ms: 100.0,
                accuracy: 0.95,
                cost_per_operation: 0.0002,
                use_cases: vec![
                    "Bulk processing".to_string(),
                    "Retrospective analysis".to_string(),
                ],
            },
        ]
    }
}
```

### EWC++ Consolidation

```rust
/// Elastic Weight Consolidation with improvements
pub struct EWCPlus {
    /// Fisher information matrix
    fisher_matrix: HashMap<String, Array2<f32>>,

    /// Parameter values at consolidation time
    consoli dated_params: HashMap<String, Array1<f32>>,

    /// Lambda (importance weight)
    lambda: f32,
}

impl EWCPlus {
    /// Consolidate after learning trajectory
    pub async fn consolidate(
        &mut self,
        trajectory: &ResolutionTrajectory,
        model: &mut NeuralModel,
    ) -> Result<ConsolidationResult> {
        // Only consolidate successful trajectories
        if trajectory.reward() < 0.5 {
            return Ok(ConsolidationResult {
                consolidated: false,
                reason: "Low reward score".to_string(),
            });
        }

        // Compute Fisher information matrix
        let fisher = self.compute_fisher_information(model, trajectory).await?;

        // Store for future loss computation
        for (param_name, fisher_val) in fisher {
            self.fisher_matrix
                .insert(param_name.clone(), fisher_val.clone());
            self.consolidated_params
                .insert(param_name, model.get_parameter(&param_name));
        }

        Ok(ConsolidationResult {
            consolidated: true,
            reason: "Successfully consolidated".to_string(),
        })
    }

    /// Compute EWC loss to prevent forgetting
    pub fn ewc_loss(&self, current_params: &HashMap<String, Array1<f32>>) -> f32 {
        let mut loss = 0.0;

        for (param_name, fisher) in &self.fisher_matrix {
            if let (Some(current), Some(consolidated)) = (
                current_params.get(param_name),
                self.consolidated_params.get(param_name),
            ) {
                // EWC loss: 0.5 * lambda * Fisher * (theta - theta*)^2
                let diff = current - consolidated;
                let param_loss = 0.5 * self.lambda
                    * fisher
                    * diff.mapv(|x| x * x).sum();

                loss += param_loss;
            }
        }

        loss
    }

    /// Compute Fisher information for parameters
    async fn compute_fisher_information(
        &self,
        model: &NeuralModel,
        trajectory: &ResolutionTrajectory,
    ) -> Result<HashMap<String, Array2<f32>>> {
        let mut fisher = HashMap::new();

        // Sample from trajectory
        let samples = trajectory.sample(100);

        // Compute gradients for each sample
        let mut gradients = Vec::new();
        for sample in &samples {
            let grad = model.compute_gradients(&sample.state, &sample.action).await?;
            gradients.push(grad);
        }

        // Fisher = E[gradient^2]
        for param_name in model.parameter_names() {
            let param_grads: Vec<_> = gradients
                .iter()
                .filter_map(|g| g.get(param_name))
                .collect();

            if !param_grads.is_empty() {
                let fisher_val = self.compute_expectation(&param_grads);
                fisher.insert(param_name.clone(), fisher_val);
            }
        }

        Ok(fisher)
    }
}
```

---

## Summary

| Learning Capability | Input | Output | SONA Mode | Storage |
|---------------------|-------|--------|-----------|---------|
| Pattern Recognition | Resolved incidents | Remediation patterns | Balanced | patterns namespace |
| Anomaly Learning | Labeled anomalies | Adaptive thresholds | RealTime | anomalies namespace |
| Topology Learning | Communication events | Dependency graph | Balanced | topology namespace |
| Runbook Extraction | Successful resolutions | Automation procedures | Research | runbooks namespace |
| Feedback Integration | Engineer overrides | Updated patterns | Balanced | feedback namespace |

---

**Document Version**: 1.0
**Last Updated**: 2026-01-18
**Author**: Learning Architecture Team
