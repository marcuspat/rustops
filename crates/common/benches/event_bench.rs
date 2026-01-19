//! Benchmarks for domain event operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustops_common::events::{DomainEvent, EventPayload, EventType, Severity};
use rustops_common::{AlertId, AnomalyId, IncidentId, MetricId, ServiceId};

fn bench_event_creation(c: &mut Criterion) {
    c.bench_function("event_creation_simple", |b| {
        b.iter(|| {
            DomainEvent::new(
                EventType::AnomalyDetected,
                EventPayload::AnomalyDetected {
                    anomaly_id: AnomalyId::new(),
                    metric_id: MetricId::new(),
                    score: 0.95,
                    confidence: 0.9,
                    explanation: "CPU spike detected".to_string(),
                },
            )
        });
    });

    c.bench_function("event_creation_with_correlation", |b| {
        let correlation_id = uuid::Uuid::new_v4();
        b.iter(|| {
            DomainEvent::new(
                EventType::AlertCreated,
                EventPayload::AlertCreated {
                    alert_id: AlertId::new(),
                    title: "High CPU usage".to_string(),
                    severity: Severity::Critical,
                    service_id: ServiceId::new(),
                },
            )
            .with_correlation(black_box(correlation_id))
        });
    });
}

fn bench_event_serialization(c: &mut Criterion) {
    let event = DomainEvent::new(
        EventType::IncidentCreated,
        EventPayload::IncidentCreated {
            incident_id: IncidentId::new(),
            title: "Service Outage".to_string(),
            severity: Severity::Major,
            alert_ids: vec![AlertId::new(), AlertId::new(), AlertId::new()],
        },
    );

    c.bench_function("event_serialize_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&event)));
    });

    c.bench_function("event_deserialize_json", |b| {
        let json = serde_json::to_string(&event).unwrap();
        b.iter(|| serde_json::from_str::<DomainEvent>(black_box(&json)));
    });
}

fn bench_event_batch(c: &mut Criterion) {
    let mut group = c.benchmark_group("event_batch");

    for size in [10, 100, 1000].iter() {
        let events: Vec<DomainEvent> = (0..*size)
            .map(|_| {
                DomainEvent::new(
                    EventType::AlertCreated,
                    EventPayload::AlertCreated {
                        alert_id: AlertId::new(),
                        title: "Test Alert".to_string(),
                        severity: Severity::Warning,
                        service_id: ServiceId::new(),
                    },
                )
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let jsons: Vec<_> = black_box(&events)
                    .iter()
                    .map(|e| serde_json::to_string(e).unwrap())
                    .collect();
                black_box(jsons)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_event_creation,
    bench_event_serialization,
    bench_event_batch
);
criterion_main!(benches);
