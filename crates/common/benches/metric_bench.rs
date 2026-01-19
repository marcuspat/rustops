//! Benchmarks for metric operations.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustops_common::{testing::MetricBuilder, Metric};
use std::collections::HashMap;

fn bench_metric_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_creation");

    group.bench_function("builder_pattern", |b| {
        b.iter(|| {
            MetricBuilder::new()
                .name("cpu_usage".to_string())
                .value(75.5)
                .label("host", "server1")
                .label("region", "us-west")
                .build()
        });
    });

    group.bench_function("direct_construction", |b| {
        b.iter(|| {
            let mut labels = HashMap::new();
            labels.insert("host".to_string(), "server1".to_string());
            labels.insert("region".to_string(), "us-west".to_string());

            Metric {
                name: "cpu_usage".to_string(),
                value: 75.5,
                labels,
                timestamp: chrono::Utc::now().timestamp(),
            }
        });
    });

    group.finish();
}

fn bench_metric_serialization(c: &mut Criterion) {
    let metric = MetricBuilder::new()
        .name("test_metric".to_string())
        .value(42.0)
        .label("key1", "value1")
        .label("key2", "value2")
        .label("key3", "value3")
        .label("key4", "value4")
        .label("key5", "value5")
        .build();

    c.bench_function("metric_serialize_json", |b| {
        b.iter(|| serde_json::to_string(black_box(&metric)));
    });

    c.bench_function("metric_deserialize_json", |b| {
        let json = serde_json::to_string(&metric).unwrap();
        b.iter(|| serde_json::from_str::<Metric>(black_box(&json)));
    });
}

fn bench_metric_labels(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_labels");

    for label_count in [1, 5, 10, 20, 50].iter() {
        let mut metric = MetricBuilder::new().name("test".to_string());
        for i in 0..*label_count {
            metric = metric.label(format!("key{}", i), format!("value{}", i));
        }
        let metric = metric.build();

        group.bench_with_input(
            BenchmarkId::from_parameter(label_count),
            label_count,
            |b, _| {
                b.iter(|| {
                    let _count = black_box(&metric).labels.len();
                });
            },
        );
    }

    group.finish();
}

fn bench_metric_aggregation(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_aggregation");

    for size in [10, 100, 1000, 10000].iter() {
        let metrics: Vec<Metric> = (0..*size)
            .map(|i| {
                MetricBuilder::new()
                    .name("cpu_usage".to_string())
                    .value(50.0 + i as f64 * 0.1)
                    .label("instance", format!("host-{}", i))
                    .build()
            })
            .collect();

        group.bench_with_input(BenchmarkId::from_parameter(size), size, |b, _| {
            b.iter(|| {
                let sum: f64 = black_box(&metrics).iter().map(|m| m.value).sum();
                black_box(sum)
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    bench_metric_creation,
    bench_metric_serialization,
    bench_metric_labels,
    bench_metric_aggregation
);
criterion_main!(benches);
