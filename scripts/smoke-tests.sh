#!/bin/bash
# Smoke tests for RustOps deployment
# Usage: ./scripts/smoke-tests.sh <environment>

set -e

ENVIRONMENT=${1:-dev}
NAMESPACE="rustops-${ENVIRONMENT}"
TIMEOUT=60
FAILED=0

echo "Running smoke tests for environment: ${ENVIRONMENT}"
echo "Namespace: ${NAMESPACE}"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

# Helper function for health checks
check_health() {
    local service=$1
    local port=$2
    local path=$3

    echo -n "Checking ${service} health..."

    local endpoint="http://${service}.${NAMESPACE}.svc.cluster.local:${port}${path}"

    if kubectl run "curl-test-${service}" \
        --image=curlimages/curl:latest \
        --rm -i --restart=Never \
        --timeout="${TIMEOUT}s \
        -- curl -f -s "${endpoint}" > /dev/null 2>&1; then
        echo -e " ${GREEN}PASSED${NC}"
        return 0
    else
        echo -e " ${RED}FAILED${NC}"
        return 1
    fi
}

# Helper function for pod status
check_pods() {
    local component=$1

    echo -n "Checking ${component} pods..."

    local ready_pods=$(kubectl get pods -n "${NAMESPACE}" \
        -l "app.kubernetes.io/component=${component}" \
        -o jsonpath='{.items[*].status.conditions[?(@.type=="Ready")].status}' 2>/dev/null || echo "")

    if [[ "$ready_pods" =~ "True" ]]; then
        echo -e " ${GREEN}PASSED${NC}"
        return 0
    else
        echo -e " ${RED}FAILED${NC}"
        return 1
    fi
}

# Helper function for logs check
check_logs_for_errors() {
    local component=$1
    local lines=100

    echo -n "Checking ${component} logs for errors..."

    local errors=$(kubectl logs -n "${NAMESPACE}" \
        -l "app.kubernetes.io/component=${component}" \
        --tail="${lines}" \
        --all-containers=true \
        2>/dev/null || echo "")

    if echo "${errors}" | grep -qi "error\|panic\|fatal"; then
        echo -e " ${YELLOW}WARNINGS FOUND${NC}"
        echo "Recent errors:"
        echo "${errors}" | grep -i "error\|panic\|fatal" | tail -5
        return 0
    else
        echo -e " ${GREEN}PASSED${NC}"
        return 0
    fi
}

# Check if namespace exists
echo -n "Checking namespace exists..."
if kubectl get namespace "${NAMESPACE}" > /dev/null 2>&1; then
    echo -e " ${GREEN}PASSED${NC}"
else
    echo -e " ${RED}FAILED${NC}"
    echo "Namespace ${NAMESPACE} does not exist"
    exit 1
fi

# Run health checks
echo "Running health checks..."
echo ""

check_pods "api" || ((FAILED++))
check_pods "agent" || ((FAILED++))
check_pods "pipeline" || ((FAILED++))

echo ""

check_health "rustops-api" "8080" "/health" || ((FAILED++))
check_health "rustops-agent" "8081" "/health" || ((FAILED++))

echo ""

# Check logs
echo "Checking logs for errors..."
echo ""

check_logs_for_errors "api"
check_logs_for_errors "agent"
check_logs_for_errors "pipeline"

echo ""

# Resource usage check
echo "Checking resource usage..."
echo ""

kubectl top pods -n "${NAMESPACE}" -l "app.kubernetes.io/name=rustops" || true

echo ""

# Summary
echo "========================================="
if [ ${FAILED} -gt 0 ]; then
    echo -e "Smoke tests: ${RED}FAILED${NC} (${FAILED} failures)"
    exit 1
else
    echo -e "Smoke tests: ${GREEN}PASSED${NC}"
    exit 0
fi
