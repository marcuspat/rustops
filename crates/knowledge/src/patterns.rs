// Pattern extraction from incidents
//
// Extracts successful remediation patterns from incident resolutions

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pattern extractor
pub struct PatternExtractor {
    min_confidence: f32,
    min_occurrences: usize,
}

impl PatternExtractor {
    /// Create new pattern extractor
    pub fn new(min_confidence: f32, min_occurrences: usize) -> Self {
        Self {
            min_confidence,
            min_occurrences,
        }
    }

    /// Extract patterns from incident
    pub async fn extract_from_incident(
        &self,
        incident: &IncidentData,
    ) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();

        // Extract symptom patterns
        if let Some(symptom) = self.extract_symptom_pattern(incident) {
            patterns.push(symptom);
        }

        // Extract resolution patterns
        if let Some(resolution) = self.extract_resolution_pattern(incident) {
            patterns.push(resolution);
        }

        // Extract action sequences
        for action_seq in self.extract_action_sequences(incident)? {
            patterns.push(action_seq);
        }

        // Filter by confidence
        patterns.retain(|p| p.confidence >= self.min_confidence);

        Ok(patterns)
    }

    fn extract_symptom_pattern(&self, incident: &IncidentData) -> Option<Pattern> {
        Some(Pattern {
            id: ulid::Ulid::new().to_string(),
            pattern_type: PatternType::Symptom,
            name: incident.title.clone(),
            description: incident.description.clone(),
            confidence: 0.8,
            occurrence_count: 1,
            service: incident.service.clone(),
            environment: incident.environment.clone(),
            severity: incident.severity,
            metadata: HashMap::new(),
            created_at: Utc::now(),
            last_seen: Utc::now(),
        })
    }

    fn extract_resolution_pattern(&self, incident: &IncidentData) -> Option<Pattern> {
        if let Some(resolution) = &incident.resolution {
            Some(Pattern {
                id: ulid::Ulid::new().to_string(),
                pattern_type: PatternType::Resolution,
                name: format!("Resolution: {}", incident.title),
                description: resolution.clone(),
                confidence: 0.9,
                occurrence_count: 1,
                service: incident.service.clone(),
                environment: incident.environment.clone(),
                severity: incident.severity,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("resolution_time_minutes".to_string(),
                        incident.resolution_time.as_secs().to_string());
                    meta
                },
                created_at: Utc::now(),
                last_seen: Utc::now(),
            })
        } else {
            None
        }
    }

    fn extract_action_sequences(&self, incident: &IncidentData) -> Result<Vec<Pattern>> {
        let mut patterns = Vec::new();

        for (i, action) in incident.actions.iter().enumerate() {
            patterns.push(Pattern {
                id: ulid::Ulid::new().to_string(),
                pattern_type: PatternType::ActionSequence,
                name: format!("Action {}: {}", i + 1, action.name),
                description: format!("{}: {}", action.name, action.description),
                confidence: action.success_rate,
                occurrence_count: 1,
                service: incident.service.clone(),
                environment: incident.environment.clone(),
                severity: incident.severity,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("action_type".to_string(), action.action_type.clone());
                    meta.insert("duration_seconds".to_string(), action.duration.as_secs().to_string());
                    meta
                },
                created_at: Utc::now(),
                last_seen: Utc::now(),
            });
        }

        Ok(patterns)
    }
}

/// Pattern extracted from incidents
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub id: String,
    pub pattern_type: PatternType,
    pub name: String,
    pub description: String,
    pub confidence: f32,
    pub occurrence_count: usize,
    pub service: String,
    pub environment: String,
    pub severity: SeverityLevel,
    pub metadata: HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}

/// Pattern types
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum PatternType {
    Symptom,
    Resolution,
    ActionSequence,
    Correlation,
    RootCause,
}

/// Pattern context for querying
#[derive(Debug, Clone)]
pub struct PatternContext {
    pub service: String,
    pub environment: String,
    pub min_confidence: f32,
}

/// Incident data for pattern extraction
#[derive(Debug, Clone)]
pub struct IncidentData {
    pub title: String,
    pub description: String,
    pub service: String,
    pub environment: String,
    pub severity: SeverityLevel,
    pub resolution: Option<String>,
    pub resolution_time: std::time::Duration,
    pub actions: Vec<Action>,
}

/// Action taken during incident resolution
#[derive(Debug, Clone)]
pub struct Action {
    pub name: String,
    pub description: String,
    pub action_type: String,
    pub success_rate: f32,
    pub duration: std::time::Duration,
}

/// Severity level
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, PartialOrd)]
pub enum SeverityLevel {
    Low,
    Medium,
    High,
    Critical,
}

// =============================================================================
// Schema SQL
// =============================================================================

pub const SCHEMA_SQL: &str = r#"
-- Tables are defined in repository.rs
"#;
