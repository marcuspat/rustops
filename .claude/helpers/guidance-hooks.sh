#!/bin/bash
# Guidance Hooks for Claude Flow V3
# Provides context and routing for Claude Code operations

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
CACHE_DIR="$PROJECT_ROOT/.claude-flow"

# Ensure cache directory exists
mkdir -p "$CACHE_DIR" 2>/dev/null || true

# Color codes
CYAN='\033[0;36m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
RESET='\033[0m'
DIM='\033[2m'

# Get command
COMMAND="${1:-help}"
shift || true

case "$COMMAND" in
    pre-edit)
        FILE_PATH="$1"
        if [[ -n "$FILE_PATH" ]]; then
            if [[ "$FILE_PATH" =~ (config|secret|credential|password|key|auth) ]]; then
                echo -e "${YELLOW}[Guidance] Security-sensitive file${RESET}"
            fi
            if [[ "$FILE_PATH" =~ ^v3/ ]]; then
                echo -e "${CYAN}[Guidance] V3 module - follow ADR guidelines${RESET}"
            fi
        fi
        exit 0
        ;;

    post-edit)
        FILE_PATH="$1"
        echo "$(date -Iseconds) edit $FILE_PATH" >> "$CACHE_DIR/edit-history.log" 2>/dev/null || true
        exit 0
        ;;

    pre-command)
        COMMAND_STR="$1"
        if [[ "$COMMAND_STR" =~ (rm -rf|sudo|chmod 777) ]]; then
            echo -e "${RED}[Guidance] High-risk command${RESET}"
        fi
        exit 0
        ;;

    route)
        TASK="$1"
        [[ -z "$TASK" ]] && exit 0

        # RustOps DDD-based routing
        if [[ "$TASK" =~ (telemetry|metrics|logs|traces|collection|ingest) ]]; then
            echo -e "${DIM}[Route] Telemetry Collection context${RESET}"
            echo -e "${CYAN}[Pattern] Kafka-based pipeline, zero-copy parsing${RESET}"
        elif [[ "$TASK" =~ (anomaly|detection|ML|prediction|pattern) ]]; then
            echo -e "${DIM}[Route] Anomaly Detection context${RESET}"
            echo -e "${CYAN}[Pattern] ONNX inference, statistical baselines${RESET}"
        elif [[ "$TASK" =~ (incident|alert|correlation|deduplication|noise) ]]; then
            echo -e "${DIM}[Route] Incident Management context${RESET}"
            echo -e "${CYAN}[Pattern] Event sourcing, CQRS, time-based grouping${RESET}"
        elif [[ "$TASK" =~ (remediation|workflow|automation|heal|fix) ]]; then
            echo -e "${DIM}[Route] Remediation context${RESET}"
            echo -e "${CYAN}[Pattern] Temporal workflows, approval gates, blast radius${RESET}"
        elif [[ "$TASK" =~ (topology|dependency|service|impact|graph) ]]; then
            echo -e "${DIM}[Route] Service Topology context${RESET}"
            echo -e "${CYAN}[Pattern] Graph database, real-time discovery${RESET}"
        elif [[ "$TASK" =~ (security|CVE|vulnerability|threat) ]]; then
            echo -e "${DIM}[Route] Security context${RESET}"
            echo -e "${CYAN}[Pattern] Zero-trust, mTLS, approval gates${RESET}"
        elif [[ "$TASK" =~ (memory|AgentDB|HNSW|vector|embedding|knowledge) ]]; then
            echo -e "${DIM}[Route] Knowledge Management context${RESET}"
            echo -e "${CYAN}[Pattern] Vector embeddings, HNSW indexing (150x-12,500x)${RESET}"
        elif [[ "$TASK" =~ (integration|adapter|external|ITSM|ServiceNow|Jira|Slack) ]]; then
            echo -e "${DIM}[Route] Integration context${RESET}"
            echo -e "${CYAN}[Pattern] Anti-corruption layer, circuit breakers${RESET}"
        elif [[ "$TASK" =~ (performance|optimize|benchmark|Flash|SIMD) ]]; then
            echo -e "${DIM}[Route] Performance Engineer${RESET}"
            echo -e "${CYAN}[Pattern] Zero-copy, arena allocation, lock-free${RESET}"
        elif [[ "$TASK" =~ (test|TDD|spec) ]]; then
            echo -e "${DIM}[Route] TDD London School${RESET}"
            echo -e "${CYAN}[Pattern] Mock-first, property-based testing${RESET}"
        fi
        exit 0
        ;;

    session-context)
        cat << 'EOF'
## RustOps AIOps Platform - DDD Development Context

**Architecture**: Domain-Driven Design with 7 Bounded Contexts
**Language**: Rust (tokio async runtime, anyhow error handling)
**Priority**: Security-first + Performance-optimized

### Bounded Contexts
1. **Telemetry Collection** - Ingest metrics/logs/traces/events (Kafka)
2. **Anomaly Detection** - ML-based pattern recognition (ONNX)
3. **Incident Management** - Alert correlation, deduplication (CQRS)
4. **Remediation** - Workflow orchestration with approval gates (Temporal)
5. **Service Topology** - Dependency discovery, impact analysis (Neo4j)
6. **Integration** - Anti-corruption layers for ITSM platforms
7. **Knowledge Management** - Runbooks, patterns, vector embeddings (HNSW)

### Performance Targets
- HNSW search: 150x-12,500x faster (pattern retrieval)
- Flash Attention: 2.49x-7.47x ML inference speedup
- Alert correlation: <500ms end-to-end (p99)
- Query response: <200ms (p95)
- Agent overhead: <1% CPU, <150MB memory
- Storage: 99.4% reduction (intelligent sampling)

### Security Requirements
- Zero-trust architecture with mTLS
- Approval gates for dangerous remediation actions
- Blast radius limits (namespace/cluster scope)
- CVE-1/2/3 mitigations (input validation, SQL injection, SSRF)
- Immutable audit logging (WORM storage)

### Active Patterns
- Event sourcing for incident history
- CQRS for read/write separation
- Vector embeddings for semantic search
- Adapter pattern for external integrations
- Repository pattern for persistence abstraction

### Code Quality Rules
- Files under 500 lines
- No hardcoded secrets (use Vault)
- Input validation at all boundaries
- Type-safe IDs (newtype pattern)
- 80% test coverage minimum
- Zero clippy warnings

### Technology Stack
- Async: tokio (full features)
- Error: anyhow + thiserror
- Metrics: QuestDB / VictoriaMetrics
- Logs: ClickHouse
- Graph: Neo4j
- ML: ONNX Runtime
- Workflow: Temporal
EOF
        exit 0
        ;;

    user-prompt)
        exit 0
        ;;

    *)
        exit 0
        ;;
esac
