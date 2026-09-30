// Pattern extraction from incidents
//
// Extracts successful remediation patterns from incident resolutions

use anyhow::Result;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Pattern extractor
///
/// `min_occurrences` compares against repetition **within one incident**
/// (action sequences repeated in `IncidentData.actions`). Symptom and
/// resolution patterns occur exactly once per incident by construction, so
/// a floor above 1 filters them out by design — cross-incident counting
/// belongs to the pattern store that aggregates extracted patterns.
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
    pub async fn extract_from_incident(&self, incident: &IncidentData) -> Result<Vec<Pattern>> {
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

        // Filter by confidence and occurrence floor
        patterns.retain(|p| {
            p.confidence >= self.min_confidence && p.occurrence_count >= self.min_occurrences
        });

        Ok(patterns)
    }

    fn extract_symptom_pattern(&self, incident: &IncidentData) -> Option<Pattern> {
        Some(Pattern {
            id: uuid::Uuid::new_v4().to_string(),
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
        incident.resolution.as_ref().map(|resolution| Pattern {
            id: uuid::Uuid::new_v4().to_string(),
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
                meta.insert(
                    "resolution_time_minutes".to_string(),
                    incident.resolution_time.as_secs().to_string(),
                );
                meta
            },
            created_at: Utc::now(),
            last_seen: Utc::now(),
        })
    }

    fn extract_action_sequences(&self, incident: &IncidentData) -> Result<Vec<Pattern>> {
        // Count repeated actions within the incident so the
        // `min_occurrences` floor compares against real repetition instead
        // of a hardcoded 1 (which silently filtered every pattern at any
        // floor above 1).
        let mut actions: HashMap<&str, (usize, &Action)> = HashMap::new();
        for action in &incident.actions {
            let entry = actions.entry(action.name.as_str()).or_insert((0, action));
            entry.0 += 1;
        }

        let mut patterns = Vec::new();
        for (name, (count, action)) in actions {
            patterns.push(Pattern {
                id: uuid::Uuid::new_v4().to_string(),
                pattern_type: PatternType::ActionSequence,
                name: format!("Action: {}", name),
                description: format!("{}: {}", action.name, action.description),
                confidence: action.success_rate,
                occurrence_count: count,
                service: incident.service.clone(),
                environment: incident.environment.clone(),
                severity: incident.severity,
                metadata: {
                    let mut meta = HashMap::new();
                    meta.insert("action_type".to_string(), action.action_type.clone());
                    meta.insert(
                        "duration_seconds".to_string(),
                        action.duration.as_secs().to_string(),
                    );
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn incident(actions: Vec<Action>) -> IncidentData {
        IncidentData {
            title: "t".to_string(),
            description: "d".to_string(),
            service: "svc".to_string(),
            environment: "prod".to_string(),
            severity: SeverityLevel::Medium,
            resolution: None,
            resolution_time: Duration::from_secs(60),
            actions,
        }
    }

    fn action(name: &str) -> Action {
        Action {
            name: name.to_string(),
            description: "d".to_string(),
            action_type: "restart".to_string(),
            success_rate: 0.9,
            duration: Duration::from_secs(5),
        }
    }

    #[tokio::test]
    async fn min_occurrences_floor_compares_real_repetition() {
        // Regression: occurrence_count used to be hardcoded to 1, so any
        // floor >= 2 silently filtered every pattern, including genuinely
        // repeated actions.
        let extractor = PatternExtractor::new(0.5, 2);

        let data = incident(vec![
            action("restart"),
            action("restart"),
            action("rollback"),
        ]);
        let patterns = extractor.extract_from_incident(&data).await.unwrap();

        assert!(
            patterns
                .iter()
                .any(|p| p.name.contains("restart") && p.occurrence_count == 2),
            "the repeated action must survive the floor of 2 with count 2: {patterns:?}"
        );
        assert!(
            !patterns.iter().any(|p| p.name.contains("rollback")),
            "single-occurrence actions must be filtered at floor 2"
        );
    }

    #[tokio::test]
    async fn floor_of_one_keeps_symptom_and_resolution() {
        let extractor = PatternExtractor::new(0.5, 1);
        let data = IncidentData {
            resolution: Some("scaled up".to_string()),
            ..incident(vec![action("restart")])
        };
        let patterns = extractor.extract_from_incident(&data).await.unwrap();
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::Symptom));
        assert!(patterns
            .iter()
            .any(|p| p.pattern_type == PatternType::Resolution));
    }
}
