#!/bin/bash
# Integration tests for RustOps
# Usage: ./scripts/integration-tests.sh <environment>

set -e

ENVIRONMENT=${1:-production}
NAMESPACE="rustops-${ENVIRONMENT}"
FAILED=0

echo "Running integration tests for environment: ${ENVIRONMENT}"
echo ""

# Color codes
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Test API endpoints
test_api_endpoint() {
    local name=$1
    local method=$2
    local path=$3
    local expected_status=$4

    echo -n "Testing ${name}..."

    local response=$(kubectl run "curl-test-${name}" \
        --image=curlimages/curl:latest \
        --rm -i --restart=Never \
        -- curl -X "${method}" \
        -s -o /dev/null \
        -w "%{http_code}" \
        "http://rustops-api.${NAMESPACE}.svc.cluster.local:8080${path}" \
        2>/dev/null || echo "000")

    if [ "${response}" = "${expected_status}" ]; then
        echo -e " ${GREEN}PASSED${NC} (status: ${response})"
        return 0
    else
        echo -e " ${RED}FAILED${NC} (expected: ${expected_status}, got: ${response})"
        return 1
    fi
}

# Test data flow
test_data_flow() {
    echo "Testing data flow from agent to API..."

    # This would typically:
    # 1. Inject test data into the agent
    # 2. Verify it's processed by the pipeline
    # 3. Confirm API can query the results

    echo -e " ${YELLOW}SKIPPED${NC} (requires test data injection)"
    return 0
}

# Test metrics
test_metrics() {
    echo -n "Testing metrics endpoint..."

    local response=$(kubectl run "curl-test-metrics" \
        --image=curlimages/curl:latest \
        --rm -i --restart=Never \
        -- curl -f -s \
        "http://rustops-pipeline.${NAMESPACE}.svc.cluster.local:9090/metrics" \
        2>/dev/null | wc -l)

    if [ "${response}" -gt 0 ]; then
        echo -e " ${GREEN}PASSED${NC} (${response} metrics lines)"
        return 0
    else
        echo -e " ${RED}FAILED${NC}"
        return 1
    fi
}

# Test service discovery
test_service_discovery() {
    echo "Testing service discovery..."

    for svc in rustops-api rustops-agent rustops-pipeline; do
        echo -n "  Resolving ${svc}..."

        if kubectl run "dns-test-${svc}" \
            --image=busybox:1.36 \
            --rm -i --restart=Never \
            -- nslookup "${svc}.${NAMESPACE}.svc.cluster.local" > /dev/null 2>&1; then
            echo -e " ${GREEN}PASSED${NC}"
        else
            echo -e " ${RED}FAILED${NC}"
            ((FAILED++))
        fi
    done
}

# Run tests
echo "========================================="
echo "API Endpoint Tests"
echo "========================================="
echo ""

test_api_endpoint "Health Check" "GET" "/health" "200" || ((FAILED++))
test_api_endpoint "Readiness" "GET" "/ready" "200" || ((FAILED++))
test_api_endpoint "Metrics" "GET" "/metrics" "200" || ((FAILED++))
test_api_endpoint "Not Found" "GET" "/nonexistent" "404" || ((FAILED++))

echo ""
echo "========================================="
echo "Service Discovery Tests"
echo "========================================="
echo ""

test_service_discovery

echo ""
echo "========================================="
echo "Data Flow Tests"
echo "========================================="
echo ""

test_data_flow

echo ""
echo "========================================="
echo "Metrics Tests"
echo "========================================="
echo ""

test_metrics || ((FAILED++))

echo ""
echo "========================================="
echo "Summary"
echo "========================================="

if [ ${FAILED} -gt 0 ]; then
    echo -e "Integration tests: ${RED}FAILED${NC} (${FAILED} failures)"
    exit 1
else
    echo -e "Integration tests: ${GREEN}PASSED${NC}"
    exit 0
fi
