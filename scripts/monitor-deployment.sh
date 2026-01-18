#!/bin/bash
# Monitor deployment for errors
# Usage: ./scripts/monitor-deployment.sh <environment> <duration_seconds>

set -e

ENVIRONMENT=${1:-production}
DURATION=${2:-300}
NAMESPACE="rustops-${ENVIRONMENT}"
INTERVAL=30

echo "Monitoring deployment for ${DURATION} seconds..."
echo "Environment: ${ENVIRONMENT}"
echo "Namespace: ${NAMESPACE}"
echo ""

END_TIME=$(($(date +%s) + DURATION))

while [ $(date +%s) -lt ${END_TIME} ]; do
    echo "========================================="
    echo "Check at: $(date)"
    echo ""

    # Check pod status
    echo "Pod Status:"
    kubectl get pods -n "${NAMESPACE}" -o wide
    echo ""

    # Check for crash loops
    CRASH_LOOPS=$(kubectl get pods -n "${NAMESPACE}" \
        -o jsonpath='{.items[?(@.status.containerStatuses[*].state.waiting.reason=="CrashLoopBackOff")].metadata.name}')

    if [ -n "${CRASH_LOOPS}" ]; then
        echo "ALERT: Pods in CrashLoopBackOff:"
        echo "${CRASH_LOOPS}"
        exit 1
    fi

    # Check error logs
    echo "Recent Errors (last 10 lines):"
    for component in api agent pipeline; do
        echo "--- ${component} ---"
        kubectl logs -n "${NAMESPACE}" \
            -l "app.kubernetes.io/component=${component}" \
            --tail=10 \
            --all-containers=true 2>/dev/null | grep -i "error\|panic\|fatal" || echo "No errors found"
    done
    echo ""

    # Check resource usage
    echo "Resource Usage:"
    kubectl top pods -n "${NAMESPACE}" --no-headers 2>/dev/null || echo "Metrics not available"
    echo ""

    REMAINING=$((END_TIME - $(date +%s)))
    echo "Next check in ${INTERVAL}s (${REMAINING}s remaining)"
    echo ""

    sleep ${INTERVAL}
done

echo "Monitoring complete. No critical issues detected."
