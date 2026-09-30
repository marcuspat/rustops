//! Benchmarks for metric operations.
//!
//! Written against the public `Metric` API (`Metric::new`/`Metric::gauge`);
//! the `testing::MetricBuilder` helper is `#[cfg(test)]`-gated and therefore
//! not visible to bench targets.

use criterion::{black_box, criterion_group, criterion_main, BenchmarkId, Criterion};
use rustops_common::{Metric, ServiceId};
use std::collections::HashMap;

fn sample_labels(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs
        .iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn bench_metric_creation(c: &mut Criterion) {
    let mut group = c.benchmark_group("metric_creation");

    group.bench_function("constructor_with_labels", |b| {
        b.iter(|| {
            let labels = sample_labels(&[("host", "server1"), ("region", "us-west")]);
            Metric::gauge("cpu_usage", 75.5, ServiceId::new(), labels)
        });
    });

    group.bench_function("constructor_empty_labels", |b| {
        b.iter(|| Metric::gauge("cpu_usage", 75.5, ServiceId::new(), HashMap::new()));
    });

    group.finish();
}

fn bench_metric_serialization(c: &mut Criterion) {
    let metric = Metric::gauge(
        "test_metric",
        42.0,
        ServiceId::new(),
        sample_labels(&[
            ("key1", "value1"),
            ("key2", "value2"),
            ("key3", "value3"),
            ("key4", "value4"),
            ("key5", "value5"),
        ]),
    );

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
        let labels: HashMap<String, String> = (0..*label_count)
            .map(|i| (format!("key{}", i), format!("value{}", i)))
            .collect();
        let metric = Metric::gauge("test", 1.0, ServiceId::new(), labels);

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
        let service_id = ServiceId::new();
        let metrics: Vec<Metric> = (0..*size)
            .map(|i| {
                Metric::gauge(
                    "cpu_usage",
                    50.0 + i as f64 * 0.1,
                    service_id,
                    sample_labels(&[("instance", "placeholder")]),
                )
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
