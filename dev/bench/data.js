window.BENCHMARK_DATA = {
  "lastUpdate": 1791386745456,
  "repoUrl": "https://github.com/marcuspat/rustops",
  "entries": {
    "Benchmark": [
      {
        "commit": {
          "author": {
            "email": "marcus@adventureonthewave.com",
            "name": "Marcus Patman",
            "username": "marcuspat"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "ce8bc4ffee72ccc5cb0382ab2f5d4af95a185f79",
          "message": "ci(bench): contents:write for the benchmark job — auto-push needs it (#19)\n\nThe default GITHUB_TOKEN is read-only; the dashboard push to gh-pages\nfails with 403 'Permission to marcuspat/rustops.git denied to\ngithub-actions[bot]' (run 36785415875). Scoped to the benchmark job\nonly.",
          "timestamp": "2026-09-30T16:44:07-06:00",
          "tree_id": "961695e575b6e3439b5bf5124837fbc05945fbd8",
          "url": "https://github.com/marcuspat/rustops/commit/ce8bc4ffee72ccc5cb0382ab2f5d4af95a185f79"
        },
        "date": 1790808519355,
        "tool": "cargo",
        "benches": [
          {
            "name": "event_creation_simple",
            "value": 1576,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "event_creation_with_correlation",
            "value": 1587,
            "range": "± 21",
            "unit": "ns/iter"
          },
          {
            "name": "event_serialize_json",
            "value": 665,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "event_deserialize_json",
            "value": 782,
            "range": "± 27",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/10",
            "value": 5948,
            "range": "± 78",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/100",
            "value": 60580,
            "range": "± 1084",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/1000",
            "value": 599643,
            "range": "± 8870",
            "unit": "ns/iter"
          },
          {
            "name": "metric_creation/constructor_with_labels",
            "value": 1196,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "metric_creation/constructor_empty_labels",
            "value": 1078,
            "range": "± 26",
            "unit": "ns/iter"
          },
          {
            "name": "metric_serialize_json",
            "value": 627,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "metric_deserialize_json",
            "value": 1094,
            "range": "± 18",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/5",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/20",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/50",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/10",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/100",
            "value": 56,
            "range": "± 1",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/1000",
            "value": 925,
            "range": "± 11",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/10000",
            "value": 9710,
            "range": "± 145",
            "unit": "ns/iter"
          }
        ]
      },
      {
        "commit": {
          "author": {
            "email": "marcus@adventureonthewave.com",
            "name": "Marcus Patman",
            "username": "marcuspat"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "534df3f808d54dcb35198f63d77bcf9857d92fd5",
          "message": "Add animated SVG banner to README (#20)",
          "timestamp": "2026-10-07T09:21:06-06:00",
          "tree_id": "35f493bc87d6badba4d5e3e604e4e0ebe829dae3",
          "url": "https://github.com/marcuspat/rustops/commit/534df3f808d54dcb35198f63d77bcf9857d92fd5"
        },
        "date": 1791386744418,
        "tool": "cargo",
        "benches": [
          {
            "name": "event_creation_simple",
            "value": 1342,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "event_creation_with_correlation",
            "value": 1359,
            "range": "± 20",
            "unit": "ns/iter"
          },
          {
            "name": "event_serialize_json",
            "value": 690,
            "range": "± 47",
            "unit": "ns/iter"
          },
          {
            "name": "event_deserialize_json",
            "value": 838,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/10",
            "value": 6092,
            "range": "± 56",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/100",
            "value": 61149,
            "range": "± 458",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/1000",
            "value": 615677,
            "range": "± 6092",
            "unit": "ns/iter"
          },
          {
            "name": "metric_creation/constructor_with_labels",
            "value": 1060,
            "range": "± 156",
            "unit": "ns/iter"
          },
          {
            "name": "metric_creation/constructor_empty_labels",
            "value": 930,
            "range": "± 2",
            "unit": "ns/iter"
          },
          {
            "name": "metric_serialize_json",
            "value": 645,
            "range": "± 3",
            "unit": "ns/iter"
          },
          {
            "name": "metric_deserialize_json",
            "value": 1061,
            "range": "± 6",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/1",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/5",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/10",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/20",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_labels/50",
            "value": 0,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/10",
            "value": 3,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/100",
            "value": 54,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/1000",
            "value": 880,
            "range": "± 23",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/10000",
            "value": 9826,
            "range": "± 545",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}