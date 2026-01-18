# Phase 2: Intelligence - Months 4-6

**Duration**: 3 months (12 weeks)
**Sprints**: 6 sprints (2 weeks each)
**Primary Goal**: ML-powered anomaly detection and alert correlation
**Key Deliverable**: 50% reduction in alert noise through intelligent correlation

---

## Executive Summary

Phase 2 transforms RustOps from a traditional monitoring tool into an intelligent AIOps platform. This phase introduces machine learning models for anomaly detection, implements sophisticated alert correlation to reduce noise, automatically discovers service topology, and provides basic root cause analysis capabilities. The deliverable is a 50% reduction in alert volume while maintaining (or improving) detection quality.

### Success Criteria
- ✅ ML models deployed with >85% precision
- ✅ Alert noise reduced by 50%
- ✅ False positive rate under 20%
- ✅ Service topology auto-discovered for 90% of services
- ✅ Root cause suggestions with >60% accuracy
- ✅ ServiceNow and Jira integrations operational

---

## Sprint Breakdown

### Sprint 7 (Weeks 13-14): ML Model Development

**Theme**: Anomaly Detection Model Training and Validation

#### Tasks

| ID | Task | Owner | Estimate | Priority |
|----|------|-------|----------|----------|
| **7.1** | Design ML model architecture | ML Lead | 16h | P0 |
| **7.2** | Prepare training dataset from Phase 1 data | ML Eng 1 | 24h | P0 |
| **7.3** | Implement time series anomaly model (LSTM) | ML Eng 2 | 32h | P0 |
| **7.4** | Implement log anomaly detection (Isolation Forest) | ML Eng 1 | 24h | P0 |
| **7.5** | Train models on historical data | ML Eng 2 | 16h | P0 |
| **7.6** | Validate model performance (precision/recall) | ML Eng 1 | 16h | P0 |
| **7.7** | Export models to ONNX format | ML Eng 2 | 8h | P0 |
| **7.8** | Create model evaluation report | ML Lead | 8h | P1 |

#### Dependencies
- Phase 1: 90 days of historical data available
- Phase 1: Labeled incident dataset

#### Deliverables
- Trained LSTM model for metric anomalies (precision >85%)
- Trained Isolation Forest for log anomalies (precision >80%)
- Models exported to ONNX format
- Model evaluation report with metrics
- Training pipeline documented

#### ML Model Specifications

**Time Series Anomaly Detection (LSTM)**
```python
# Model architecture
Input: [batch_size, time_steps=60, features=20]
LSTM Layer 1: 128 units, return_sequences=True
Dropout: 0.2
LSTM Layer 2: 64 units, return_sequences=False
Dropout: 0.2
Dense: 32 units, ReLU
Output: 1 unit, Sigmoid (anomaly probability)

Training:
- Optimizer: Adam (lr=0.001)
- Loss: Binary Cross-Entropy
- Epochs: 100 with early stopping
- Validation split: 20%
- Class weights: balanced

Performance targets:
- Precision: >85%
- Recall: >80%
- F1 Score: >0.82
- Inference: <10ms per prediction
```

**Log Anomaly Detection (Isolation Forest)**
```python
# Model architecture
Features: TF-IDF vectors (ngram 1-3, max_features=5000)
Isolation Forest:
- n_estimators: 100
- contamination: 0.1
- max_samples: 256

Performance targets:
- Precision: >80%
- Recall: >75%
- F1 Score: >0.77
- Inference: <5ms per log entry
```

---

### Sprint 8 (Weeks 15-16): ONNX Integration

**Theme**: ML Inference Engine in Rust

#### Tasks

| ID | Task | Owner | Estimate | Priority |
|----|------|-------|----------|----------|
| **8.1** | Add ONNX Runtime dependency to Rust project | Rust Eng | 2h | P0 |
| **8.2** | Implement ONNX model loader | Rust Eng 1 | 12h | P0 |
| **8.3** | Create inference engine interface | Rust Eng 2 | 16h | P0 |
| **8.4** | Implement batch prediction API | Rust Eng 1 | 12h | P0 |
| **8.5** | Add model versioning support | Rust Eng 2 | 8h | P1 |
| **8.6** | Implement model hot-reloading | Rust Eng 1 | 8h | P1 |
| **8.7** | Write inference performance tests | QA Eng | 12h | P0 |
| **8.8** | Create ONNX integration guide | Tech Writer | 6h | P1 |

#### Dependencies
- Sprint 7: ONNX models available

#### Deliverables
- ONNX Runtime integration in Rust
- Inference engine with <10ms latency
- Batch prediction support (up to 100 samples)
- Model versioning and hot-reload
- Performance benchmarks

#### Code Structure

```rust
// src/core/src/inference/mod.rs

use ort::{Environment, Session, SessionInputs};
use ndarray::Array2;

pub struct Model {
    session: Session,
    name: String,
    version: String,
}

impl Model {
    pub fn load(path: &Path, name: &str) -> Result<Self> {
        let environment = Environment::builder()
            .with_name("rustops-inference")
            .build()?;

        let session = environment
            .new_session_builder()?
            .with_model_from_file(path)?;

        Ok(Self {
            session,
            name: name.to_string(),
            version: Self::extract_version(&session)?,
        })
    }

    pub fn predict(&self, input: Array2<f32>) -> Result<Vec<f32>> {
        let inputs = SessionInputs::try_from(input)?;
        let outputs = self.session.run(inputs)?;

        Ok(outputs[0].try_extract()?)
    }

    pub fn predict_batch(&self, inputs: Vec<Array2<f32>>) -> Result<Vec<Vec<f32>>> {
        inputs.into_iter()
            .map(|input| self.predict(input))
            .collect()
    }
}
```

#### Performance Targets
- Model load: <1s
- Single prediction: <10ms
- Batch prediction (100): <100ms
- Memory: <100MB per model
- Concurrency: 100+ simultaneous predictions

---

### Sprint 9 (Weeks 17-18): Alert Correlation

**Theme**: Intelligent Alert Grouping and Deduplication

#### Tasks

| ID | Task | Owner | Estimate | Priority |
|----|------|-------|----------|----------|
| **9.1** | Design correlation algorithm | ML Lead | 12h | P0 |
| **9.2** | Implement time-based grouping | Rust Eng 1 | 16h | P0 |
| **9.3** | Add topology-aware clustering | Rust Eng 2 | 20h | P0 |
| **9.4** | Create alert deduplication engine | Rust Eng 1 | 16h | P0 |
| **9.3** | Implement ML-based correlation | ML Eng 1 | 20h | P0 |
| **9.4** | Add alert severity scoring | Rust Eng 2 | 12h | P1 |
| **9.5** | Write correlation tests | QA Eng | 16h | P0 |
| **9.6** | Create correlation documentation | Tech Writer | 8h | P1 |

#### Dependencies
- Sprint 8: Inference engine operational
- Sprint 11: Service topology available

#### Deliverables
- Alert correlation engine with 5-minute window
- Topology-aware clustering
- ML-based similarity scoring
- Alert deduplication (90% reduction in duplicates)
- Severity scoring algorithm

#### Correlation Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│              Alert Correlation Pipeline                      │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  1. INGESTION                                               │
│     Raw alerts → Normalize → Enrich (topology, history)     │
│                                                             │
│  2. TEMPORAL GROUPING                                       │
│     Time window: 5 minutes                                  │
│     Group by: Service + Alert Type + Similarity             │
│                                                             │
│  3. TOPOLOGY CLUSTERING                                     │
│     Build dependency graph                                  │
│     Find connected components                               │
│     Identify alert storms (cascade failures)                │
│                                                             │
│  4. ML-BASED CORRELATION                                    │
│     Feature extraction:                                     │
│       - Time proximity (0-1)                                │
│       - Service distance (0-1)                              │
│       - Message similarity (0-1)                            │
│       - Historical co-occurrence (0-1)                      │
│     Similarity score = weighted sum                         │
│     Correlate if score > threshold (0.7)                    │
│                                                             │
│  5. DEDUPLICATION                                           │
│     Within cluster: keep highest severity                   │
│     Mark as duplicate of primary alert                      │
│                                                             │
│  6. SEVERITY SCORING                                        │
│     Factors:                                                │
│       - Affected services count                             │
│       - User impact (yes/no)                                │
│       - Business criticality                                │
│       - Historical resolution time                          │
│     Final severity: P0/P1/P2/P3/P4                          │
│                                                             │
│  7. OUTPUT                                                  │
│     Clustered incidents                                     │
│     Root cause hypothesis                                   │
│     Recommended actions                                     │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### Performance Targets
- Correlation latency: <200ms per alert
- Reduction in alert volume: 50%
- Duplicate detection precision: >95%
- Correlation accuracy: >75%

---

### Sprint 10 (Weeks 19-20): Service Topology

**Theme**: Automatic Service Dependency Discovery

#### Tasks

| ID | Task | Owner | Estimate | Priority |
|----|------|-------|----------|----------|
| **10.1** | Design topology data model | Rust Eng 1 | 8h | P0 |
| **10.2** | Implement Kubernetes topology discovery | Rust Eng 2 | 20h | P0 |
| **10.3** | Add AWS service map discovery | Rust Eng 1 | 16h | P0 |
| **10.4** | Create topology graph database | Rust Eng 2 | 16h | P0 |
| **10.5** | Implement topology diff detection | Rust Eng 1 | 12h | P1 |
| **10.6** | Build topology visualization API | Rust Eng 2 | 12h | P1 |
| **10.7** | Write topology tests | QA Eng | 12h | P0 |
| **10.8** | Create topology documentation | Tech Writer | 6h | P1 |

#### Dependencies
- Sprint 4: Cloud integrations
- Phase 1: Kubernetes metrics available

#### Deliverables
- Automatic service graph discovery
- Topology visualization in dashboard
- Change detection (dependency changes)
- Impact analysis (what breaks if X fails)
- Topology export (GraphML, JSON)

#### Topology Discovery Sources

| Source | Data Collected | Frequency |
|--------|----------------|-----------|
| **Kubernetes** | Pods, Services, Ingress, NetworkPolicies | Every 5 min |
| **AWS** | VPC, ELB, RDS, Lambda, API Gateway | Every 10 min |
| **Traces** | Service-to-service calls | Real-time |
| **Logs** | API call patterns | Every hour |
| **Config** | Terraform state, CloudFormation | On change |

#### Graph Schema

```rust
// src/types/src/topology.rs

use petgraph::graph::{DiGraph, NodeIndex};

#[derive(Debug, Clone)]
pub struct ServiceNode {
    pub id: String,
    pub name: String,
    pub service_type: ServiceType,
    pub criticality: Criticality,
    pub health: HealthStatus,
    pub metadata: HashMap<String, String>,
}

#[derive(Debug, Clone)]
pub struct ServiceEdge {
    pub from: NodeIndex,
    pub to: NodeIndex,
    pub edge_type: EdgeType,
    pub strength: f32,  // 0-1, based on call frequency
    pub latency_ms: Option<f64>,
}

#[derive(Debug, Clone)]
pub enum ServiceType {
    ApiGateway,
    Microservice,
    Database,
    Cache,
    Queue,
    ExternalService,
}

#[derive(Debug, Clone)]
pub enum EdgeType {
    Synchronous,  // HTTP, gRPC
    Asynchronous, // Queue, Pub/Sub
    Dependency,   // Infrastructure
}
```

#### Performance Targets
- Discovery cycle: <5 minutes for full graph
- Graph size: Support 10K+ nodes, 100K+ edges
- Query latency: <100ms for path analysis
- Visualization: <2s render for 1K nodes

---

### Sprint 11 (Weeks 21-22): Root Cause Analysis

**Theme**: Basic Causal Inference and Hypothesis Generation

#### Tasks

| ID | Task | Owner | Estimate | Priority |
|----|------|-------|----------|----------|
| **11.1** | Design causal inference framework | ML Lead | 16h | P0 |
| **11.2** | Implement temporal correlation analysis | ML Eng 1 | 20h | P0 |
| **11.3** | Create topology-based RCA algorithm | Rust Eng 1 | 20h | P0 |
| **11.4** | Add historical pattern matching | ML Eng 2 | 16h | P0 |
| **11.5** | Implement hypothesis ranking | ML Eng 1 | 12h | P1 |
| **11.6** | Build RCA explanation UI | Frontend | 16h | P1 |
| **11.7** | Write RCA tests | QA Eng | 16h | P0 |
| **11.8** | Create RCA documentation | Tech Writer | 8h | P1 |

#### Dependencies
- Sprint 9: Alert correlation
- Sprint 10: Service topology
- Sprint 8: ML inference

#### Deliverables
- Root cause hypothesis generator
- Causal graph visualization
- Historical incident matching
- Explanation generation for RCA
- Confidence scores for hypotheses

#### RCA Algorithm

```
┌─────────────────────────────────────────────────────────────┐
│          Root Cause Analysis Pipeline                        │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  INPUT: Clustered incident with correlated alerts           │
│                                                             │
│  1. TEMPORAL ANALYSIS                                       │
│     Find earliest alert in cluster                          │
│     Identify preceding anomalies                           │
│     Calculate time lags between events                      │
│                                                             │
│  2. TOPOLOGICAL ANALYSIS                                    │
│     Build causal graph from service topology                │
│     Identify upstream dependencies                          │
│     Find single points of failure                           │
│                                                             │
│  3. HISTORICAL PATTERN MATCHING                             │
│     Search vector DB for similar clusters                   │
│     Retrieve past root causes                               │
│     Calculate similarity score                              │
│                                                             │
│  4. CAUSAL INFERENCE                                        │
│     Apply PC algorithm (Peter-Clark)                        │
│     Build causal DAG from observational data                │
│     Identify direct vs indirect causes                     │
│                                                             │
│  5. HYPOTHESIS GENERATION                                   │
│     Generate root cause candidates:                         │
│       - Topological: Service X is dependency                │
│       - Temporal: Alert Y preceded all others               │
│       - Historical: 80% similar to incident Z               │
│                                                             │
│  6. RANKING                                                 │
│     Score each hypothesis:                                  │
│       - Temporal priority: 0-3                             │
│       - Topological centrality: 0-2                         │
│       - Historical confidence: 0-3                          │
│       - Data quality: 0-2                                  │
│     Total: 0-10                                             │
│                                                             │
│  7. EXPLANATION GENERATION                                  │
│     "Service A (API Gateway) is the likely root cause       │
│      because:                                               │
│      - Alerts started 2m before other services              │
│      - It's upstream of 5 other failing services            │
│      - 3 similar incidents in past month                    │
│      Confidence: 7/10"                                      │
│                                                             │
│  OUTPUT: Ranked hypotheses with explanations                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### Performance Targets
- RCA generation: <500ms per incident
- Hypothesis accuracy: >60% top-1, >80% top-3
- Historical match: <100ms for vector search
- Explanation clarity: >70% user satisfaction

---

### Sprint 12 (Weeks 23-24): ITSM Integration

**Theme**: ServiceNow and Jira Bidirectional Sync

#### Tasks

| ID | Task | Owner | Estimate | Priority |
|----|------|-------|----------|----------|
| **12.1** | Design ITSM integration architecture | Rust Eng 1 | 8h | P0 |
| **12.2** | Implement ServiceNow REST client | Rust Eng 2 | 16h | P0 |
| **12.3** | Create Jira API integration | Rust Eng 1 | 16h | P0 |
| **12.4** | Build incident sync engine | Rust Eng 2 | 20h | P0 |
| **12.5** | Add webhook handlers for status updates | Rust Eng 1 | 12h | P0 |
| **12.6** | Implement CMDB synchronization | Rust Eng 2 | 16h | P1 |
| **12.7** | Write integration tests | QA Eng | 16h | P0 |
| **12.8** | Create integration setup guide | Tech Writer | 10h | P1 |

#### Dependencies
- Sprint 9: Alert correlation and incident clustering
- Sprint 11: Root cause analysis

#### Deliverables
- ServiceNow incident creation and sync
- Jira issue creation and sync
- Bidirectional status updates
- CMDB topology import/export
- Webhook endpoints for ITSM platforms
- Integration documentation

#### Integration Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                 ITSM Integration Flow                       │
├─────────────────────────────────────────────────────────────┤
│                                                             │
│  RustOps ──▶ ServiceNow                                    │
│    │         │                                             │
│    │         ├─ Create incident                            │
│    │         ├─ Update status (In Progress, Resolved)      │
│    │         ├─ Add RCA notes                              │
│    │         ├─ Attach topology graph                      │
│    │         └─ Sync CMDB topology                         │
│    │                                                       │
│  RustOps ──▶ Jira                                          │
│    │         │                                             │
│    │         ├─ Create issue                               │
│    │         ├─ Add comments (RCA, timeline)               │
│    │         ├─ Transition workflow                        │
│    │         ├─ Link to related issues                     │
│    │         └─ Update sprint                              │
│    │                                                       │
│  ◀──── Webhooks                                           │
│    │                                                       │
│    ├─ ServiceNow: Incident status changed                  │
│    ├─ ServiceNow: Assigned group changed                  │
│    ├─ Jira: Issue status transition                       │
│    ├─ Jira: Comment added                                 │
│    └─ Jira: Issue resolved                                │
│                                                             │
└─────────────────────────────────────────────────────────────┘
```

#### API Mappings

| RustOps Field | ServiceNow Field | Jira Field |
|---------------|------------------|------------|
| incident_id | sys_id | key |
| title | short_description | summary |
| description | description | description |
| severity | priority | priority |
| state | state | status |
| assigned_to | assigned_to | assignee |
| service | cmdb_ci | components |
| rca_hypothesis | problem_notes | customfield_10000 |

---

## Technical Architecture

### Component Diagram

```
┌─────────────────────────────────────────────────────────────────────┐
│                        Phase 2 Architecture                          │
├─────────────────────────────────────────────────────────────────────┤
│                                                                      │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                   ML Training Pipeline                      │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐     │    │
│  │  │  Historical  │  │   Python     │  │    ONNX      │     │    │
│  │  │    Data      │──▶   Training   │──▶   Models     │     │    │
│  │  │ (ClickHouse) │  │  (PyTorch)   │  │   Export     │     │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘     │    │
│  └────────────────────────────────────────────────────────────┘    │
│                              │                                       │
│                              ▼                                       │
│  ┌────────────────────────────────────────────────────────────┐    │
│  │                    RustOps Core (Enhanced)                  │    │
│  │                                                              │    │
│  │  ┌──────────────┐  ┌──────────────┐  ┌──────────────┐      │    │
│  │  │   ONNX       │  │   Alert      │  │  Service     │      │    │
│  │  │  Inference   │  │ Correlation  │  │  Topology    │      │    │
│  │  │   Engine     │  │   Engine     │  │   Engine     │      │    │
│  │  └──────────────┘  └──────────────┘  └──────────────┘      │    │
│  │         │                  │                  │             │    │
│  │         └──────────────────┼──────────────────┘             │    │
│  │                            ▼                                │    │
│  │  ┌─────────────────────────────────────────────────────┐  │    │
│  │  │         Root Cause Analysis Engine                   │  │    │
│  │  └─────────────────────────────────────────────────────┘  │    │
│  └────────────────────────────────────────────────────────────┘    │
│                              │                                       │
│          ┌───────────────────┼───────────────────┐                  │
│          ▼                   ▼                   ▼                  │
│  ┌─────────────┐     ┌─────────────┐     ┌─────────────┐           │
│  │ ServiceNow  │     │    Jira     │     │  Dashboard   │           │
│  │   / Jira    │     │  / Service  │     │  (Enhanced)  │           │
│  │  (Sync)     │     │    Now      │     │              │           │
│  └─────────────┘     └─────────────┘     └─────────────┘           │
│                                                                      │
└─────────────────────────────────────────────────────────────────────┘
```

---

## ML Model Operations

### Model Lifecycle

```
Training → Validation → ONNX Export → Deployment → Monitoring → Retraining
   ↑                                                            │
   └─────────────────────── Performance Degradation ────────────┘
```

### Monitoring Metrics

| Metric | Target | Alert Threshold |
|--------|--------|-----------------|
| **Precision** | >85% | <80% |
| **Recall** | >80% | <75% |
| **F1 Score** | >0.82 | <0.78 |
| **Inference Latency** | <10ms | >20ms |
| **Prediction Distribution** | Stable | Drift >20% |

### Retraining Triggers
- Weekly scheduled retraining
- Model precision drops below 80%
- New service patterns detected
- Labeled data increases by >10%

---

## Risk Mitigation (Phase 2)

| Risk | Impact | Mitigation | Owner |
|------|--------|-----------|-------|
| **ML model accuracy insufficient** | HIGH | Ensemble models; human-in-loop; continuous retraining | ML Lead |
| **False positives causing distrust** | HIGH | Conservative thresholds; gradual rollout; feedback loop | ML Eng |
| **Topological discovery incomplete** | MEDIUM | Multi-source fusion; manual annotation tools | Rust Eng |
| **ITSM integration complexity** | MEDIUM | Plugin architecture; prioritize top 2 platforms | Rust Eng |
| **Model drift in production** | MEDIUM | Continuous monitoring; automated retraining pipeline | ML Eng |

---

## Definition of Done

Each task is complete when:
- ✅ Code reviewed and approved
- ✅ Unit tests (>90% coverage)
- ✅ Integration tests passing
- ✅ Model performance validated
- ✅ Documentation updated
- ✅ Performance benchmarks met
- ✅ Security scan clean

Phase 2 is complete when:
- ✅ All 6 sprints delivered
- ✅ Alert noise reduced by 50%
- ✅ ML models in production with >85% precision
- ✅ Service topology auto-discovered
- ✅ ITSM integrations operational
- ✅ RCA hypotheses >60% accurate
- ✅ Stakeholder sign-off

---

## Next Phase Transition

### Handoff to Phase 3
- ML models stable and monitored
- Alert correlation baseline established
- Service topology comprehensive
- ITSM integrations tested
- Root cause patterns documented
- User feedback on accuracy collected

### Phase 3 Prerequisites
- ML inference latency <10ms
- Alert clustering accuracy >75%
- Topology graph for all services
- ITSM webhook endpoints functional
- Historical remediation data available

---

**Document Navigation:**
- [← Roadmap Overview](./README.md)
- [← Phase 1: Foundation](./02-phase-1-foundation.md)
- [Phase 3: Automation →](./04-phase-3-automation.md)
