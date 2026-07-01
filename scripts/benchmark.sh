#!/bin/bash
set -e

echo "=== OrbitLogs Performance Benchmark Suite ==="

# Check k6
command -v k6 >/dev/null 2>&1 || { echo >&2 "k6 is required but not installed. Aborting."; exit 1; }

mkdir -p benchmarks/reports

run_benchmark() {
  RATE=$1
  echo "------------------------------------------------"
  echo "Running load test at ${RATE} events/sec..."
  echo "------------------------------------------------"
  k6 run -e TARGET_RATE=${RATE} -e DURATION=20s benchmarks/load_test.js --summary-export=benchmarks/reports/summary_${RATE}.json
}

# Run sequential benchmarks
run_benchmark 1000
run_benchmark 5000
run_benchmark 10000

echo "================================================"
echo "Benchmarks completed! Reports saved in benchmarks/reports/"
echo "================================================"
