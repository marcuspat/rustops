window.BENCHMARK_DATA = {
  "lastUpdate": 1792019146143,
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
      },
      {
        "commit": {
          "author": {
            "email": "1175997+marcuspat@users.noreply.github.com",
            "name": "Marcus Patman",
            "username": "marcuspat"
          },
          "committer": {
            "email": "noreply@github.com",
            "name": "GitHub",
            "username": "web-flow"
          },
          "distinct": true,
          "id": "660d53b94271572559e7fba6dedd1496f8f91ae0",
          "message": "Make the agent real: Prometheus wire format, env config, Neo4j honesty (#22)\n\n* fix(integration): speak the real Prometheus wire format; agent env config; Neo4j stub refuses\n\nPrometheus adapter (telemetry): real Prometheus always encodes sample\nvalues as JSON strings (\"1\", \"NaN\", \"+Inf\"), never bare numbers.\nThe response structs declared Vec<(f64, f64)> / bare f64, so every\nquery against a real server failed deserialization after 5 retries.\nThe hermetic mock fed numeric values - a shape no real server emits -\nwhich kept the bug invisible to CI. Add a SampleValue type that\naccepts both strings and numbers, fix the vector shape to the\n[timestamp, \"value\"] pair the API actually sends, and update the\nwiremock fixtures to the real dialect (including NaN and a\nvector-typed range response).\n\nPrometheus adapter (legacy src/prometheus.rs): PrometheusData now\nrenames result_type to the API's camelCase resultType; its test\nfixture updated to match.\n\nAgent: read PROMETHEUS_URL and PROMETHEUS_INTERVAL_SECS from the\nenvironment (README claimed PROMETHEUS_URL; main() hardcoded\ndefaults). Malformed or non-positive values fall back with a warning.\n\nNeo4j store: connect() now refuses with an error instead of logging\n\"connection established (stubbed)\" and returning Ok - a silent Ok\nmade writes \"succeed\" and reads return empty. This matches the README\npromise that the stub \"refuses to connect rather than pretending\";\nthe test that pinned the old behavior now asserts the refusal.\n\nREADME: document both agent env vars; replace the stale reqwest 0.11\nsecurity-debt section (cleared by #21).\n\n* fix(agent): Rust format spec, not Python {!r}\n\n* fix(integration): move Utc import into test module (unused in lib now)\n\n* style(agent): tests module at end of file (clippy items_after_test_module)\n\n* fix(topology): default neo4j_uri empty (in-memory); explicit Neo4j fails loudly\n\nTopologyConfig::default() shipped bolt://localhost:7687, so default\nconstruction hit the now-refusing Neo4j stub. Default to in-memory\n(empty URI) - the only real persistence today - and pin the new\ncontract with a service-level test: explicit Neo4j config fails\nconstruction with a 'not implemented' error.\n\nREADME: TopologyConfig has no prometheus_url field (the doc named one\nthat does not exist); describe the neo4j_uri field as it behaves.\n\n* fix(topology): expect_err needs Debug on Ok type; match instead\n\n* fix(security): redact URL credentials in logs; keep ms timestamp precision\n\nGate round 1 (REVISE) findings:\n\n- The agent's new config log line and the adapter's initialize log\n  printed the configured URL verbatim; a URL like\n  http://user:pass@prometheus:9090 would leak basic-auth credentials\n  into logs. Redact userinfo as ***@ in both places (authority-only\n  match, so an '@' in a path or query never triggers redaction).\n  Unit-tested for userinfo, userinfo+path, userinfo-free, malformed,\n  and '@'-in-query inputs.\n\n- Prometheus float-second timestamps were truncated to whole seconds\n  (ts as i64); parse via from_timestamp_millis so sub-second scrape\n  timestamps survive. Tests now pin millisecond precision.\n\n- README: name the neo4j_uri/Prometheus-discovery overlap as a\n  pre-existing naming wart instead of describing it neutrally.\n\n* fix(integration): redacted_url is an inherent method, not a trait member\n\n* fix(agent): redact the pre-existing 'Prometheus URL' log line too\n\nLive run with credentials embedded in PROMETHEUS_URL caught a third\nleak the code-level pass missed: Agent::initialize logged the URL\nverbatim. Same redaction helper as the config line.\n\n* test: cover redaction edge cases + default in-memory topology service\n\nGate round 2 (APPROVED) noted missing coverage; the ?/# cases also\nexposed a real hole - authority detection stopped only at '/', so a\nquery or fragment before any slash was treated as authority and an\n'@' inside it would trigger a bogus redaction. Authority now ends at\n'/', '?' or '#' in both redaction copies; tests pin query, fragment,\nno-scheme, and query+fragment-after-userinfo cases. Plus a test that\nTopologyConfig::default() (empty neo4j_uri) builds the in-memory\nservice cleanly.\n\n* style: array pattern for authority-end scan (clippy manual_pattern_char_comparison)\n\n* fix(integration): round float timestamps to ms, don't truncate\n\nGate round 3 (REVISE): 1234567890.123 * 1000.0 is 1234567890122.9998\nin f64; 'as i64' truncation dropped a whole millisecond. Round before\ncasting; unit test pins .123, .999 and exact-second fractions.\n\n* chore: cargo fmt; rustls 0.23.46 for RUSTSEC-2026-0346..0349\n\nLinting job: cargo fmt --all over the new code (CI checks\nformatting; my verification loop ran build/clippy/test but not fmt).\n\nSecurity Audit job: four new rustls advisories (RUSTSEC-2026-0346,\n0347, 0348, 0349 - incl. the ECH early-write flaw, low severity)\nlanded in the advisory DB after Oct 12 and flag the locked 0.23.45.\nLockfile-only bump to the patched 0.23.46; no code change.\n\n* style: finish cargo fmt (previous run was SIGPIPE-killed by head mid-run)\n\n* fix(agent): reject schemeless PROMETHEUS_URL without logging it\n\nGate round 4 (REVISE, P2): from_env accepted any non-empty string, so\na malformed value like 'user:pass@host:9090' (no scheme) could carry\ncredentials past the redactor - which needs a scheme to find the\nauthority - and into the startup log. Reject values that don't start\nwith http:// or https://, warn without echoing the value (the bad\nstring itself may be the secret), and fall back to the default.\nreqwest requires absolute URLs anyway, so the old behavior only ever\nproduced failing scrapes. Test pins the credentials-shaped fallback.\n\n* fix(agent): parse PROMETHEUS_URL before accepting it\n\nGate round 5 (REVISE, P2): the prefix check accepted degenerate values\n('http://' with no host, 'https://host:invalid' with a bad port).\nParse with reqwest::Url (already in the workspace lockfile) and accept\nonly absolute http(s) URLs with a host; reject everything else without\nechoing the value. Tests pin the schemeless-credentials, no-host,\nbad-port and wrong-scheme fallbacks.\n\n* chore: lockfile records rustops-agent direct reqwest dep\n\n* fix(security): normalize accepted URLs; parse-based redaction\n\nGate round 6 (REVISE, HIGH): Url::parse accepts noncanonical\nscheme-special forms like 'http:user:pass@prometheus:9090'; the\nconfig stored the original string, and both redactors require a\nliteral '://', so that accepted input reached logs with credentials.\n\n- from_env stores the parsed URL's normalized serialization\n  ('http://user:pass@host'), which the redactors can always find.\n- redact_url_credentials (agent) and redacted_url (adapter) now\n  parse first and rebuild via set_username(\"***\")/set_password(None),\n  covering every form Url accepts; the string-authority heuristic\n  remains as fallback for unparseable input.\n- Regression tests: noncanonical input round-trips through\n  from_env into a redactable URL, and the redactor hides\n  credentials in the noncanonical form directly.\n\n* test: expect canonical trailing slash from Url serialization\n\n* test: two more canonical-serialization expectations\n\n* fix(topology): redact credentials from the Neo4j refusal error\n\nGate round 7 (REVISE, P2): the refusal message embedded the raw URI,\nso bolt://user:secret@host put the password into the propagated error\n(and any log that prints it). Redact userinfo via an authority-bounded\nmatch; test pins that a credential-bearing URI yields an error with\nthe host visible and the password absent.\n\n* fix(topology): redact schemeless URIs in the Neo4j refusal error\n\nGate round 8a flagged schemeless inputs; 8b passed the same commit,\nso the gate is non-deterministic here — fix the real gap anyway.\nA URI like 'neo4j:secret@host:7687' has no '://' for the authority\nmatch, so treat schemeless strings as authority-shaped from the start\n(over-redaction of a malformed URI is harmless; echoing credentials\nis not). Test pins the schemeless case.",
          "timestamp": "2026-10-14T17:01:37-06:00",
          "tree_id": "8d440204ea62185b9a0a6a26c6079355c5997f28",
          "url": "https://github.com/marcuspat/rustops/commit/660d53b94271572559e7fba6dedd1496f8f91ae0"
        },
        "date": 1792019145540,
        "tool": "cargo",
        "benches": [
          {
            "name": "event_creation_simple",
            "value": 866,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "event_creation_with_correlation",
            "value": 890,
            "range": "± 24",
            "unit": "ns/iter"
          },
          {
            "name": "event_serialize_json",
            "value": 349,
            "range": "± 25",
            "unit": "ns/iter"
          },
          {
            "name": "event_deserialize_json",
            "value": 494,
            "range": "± 9",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/10",
            "value": 3272,
            "range": "± 138",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/100",
            "value": 32624,
            "range": "± 1300",
            "unit": "ns/iter"
          },
          {
            "name": "event_batch/1000",
            "value": 319542,
            "range": "± 5994",
            "unit": "ns/iter"
          },
          {
            "name": "metric_creation/constructor_with_labels",
            "value": 688,
            "range": "± 19",
            "unit": "ns/iter"
          },
          {
            "name": "metric_creation/constructor_empty_labels",
            "value": 602,
            "range": "± 10",
            "unit": "ns/iter"
          },
          {
            "name": "metric_serialize_json",
            "value": 345,
            "range": "± 17",
            "unit": "ns/iter"
          },
          {
            "name": "metric_deserialize_json",
            "value": 587,
            "range": "± 12",
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
            "value": 1,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/100",
            "value": 22,
            "range": "± 0",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/1000",
            "value": 445,
            "range": "± 12",
            "unit": "ns/iter"
          },
          {
            "name": "metric_aggregation/10000",
            "value": 5073,
            "range": "± 112",
            "unit": "ns/iter"
          }
        ]
      }
    ]
  }
}