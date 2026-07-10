#!/bin/bash
# Compare fast_kv vs Redis with workloads that actually stress the server.
#
# Prerequisites:
#   redis-server on :6379
#   fast_kv on :9736  (cargo run --release --features local-logs)
#
# Usage:
#   ./tests/cmp.sh          # full suite
#   ./tests/cmp.sh quick    # sanity check only
#   ./tests/cmp.sh stress   # pipelined + large payloads

set -euo pipefail

REDIS_PORT=6379
FASTKV_PORT=9736
RUNS=${RUNS:-3}

check_server() {
  local port=$1 name=$2
  if ! redis-cli -p "$port" PING >/dev/null 2>&1; then
    echo "ERROR: $name not reachable on port $port"
    echo "  Redis:  redis-server"
    echo "  fast_kv: MAX_KEYS=1000000 EVICTION_POLICY=NoEviction cargo run --release --features local-logs"
    exit 1
  fi
}

section() {
  echo ""
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
  echo "  $1"
  echo "━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━"
}

# Run redis-benchmark N times and print each result.
bench() {
  local port=$1
  shift
  local args=("$@")
  for _ in $(seq 1 "$RUNS"); do
    redis-benchmark -p "$port" "${args[@]}" 2>&1 | grep -v '^WARNING: Could not fetch server CONFIG$' || true
    echo ""
  done
}

compare() {
  local label=$1
  shift
  local args=("$@")
  section "$label"
  echo ">>> Redis (:$REDIS_PORT)"
  bench "$REDIS_PORT" "${args[@]}"
  echo ">>> fast_kv (:$FASTKV_PORT)"
  bench "$FASTKV_PORT" "${args[@]}"
}

# ── 1. Quick sanity (original workload — likely client-bound) ──────────────
run_quick() {
  compare "Quick sanity (50 clients, no pipeline)" \
    -t ping,set,get -n 100000 -c 50 -q
}

# ── 2. Pipelined throughput — stresses server, not just the client ─────────
run_pipeline() {
  for pipeline in 1 8 16 32; do
    compare "Pipelined throughput (50 clients, -P $pipeline)" \
      -t set,get -n 500000 -c 50 -P "$pipeline" -q
  done
}

# ── 3. Large payloads — exposes alloc/copy cost ────────────────────────────
run_payload() {
  for size in 64 256 1024 4096; do
    compare "Large values (256-byte keys space, value size ${size}B)" \
      -t set,get -n 200000 -c 50 -P 16 -d "$size" -r 100000 -q
  done
}

# ── 4. Client ramp — find where throughput diverges ────────────────────────
run_clients() {
  section "Client ramp (pipelined SET/GET, -P 16)"
  for clients in 10 50 100 200; do
    echo "--- clients=$clients ---"
    echo ">>> Redis"
    redis-benchmark -p "$REDIS_PORT" -t set,get -n 200000 -c "$clients" -P 16 -q 2>&1 \
      | grep -v '^WARNING: Could not fetch server CONFIG$' || true
    echo ">>> fast_kv"
    redis-benchmark -p "$FASTKV_PORT" -t set,get -n 200000 -c "$clients" -P 16 -q 2>&1 \
      | grep -v '^WARNING: Could not fetch server CONFIG$' || true
    echo ""
  done
}

# ── 5. Latency distribution — single-digit-ms resolution ───────────────────
run_latency() {
  section "Latency distribution (10 clients, GET only)"
  echo ">>> Redis"
  redis-benchmark -p "$REDIS_PORT" -t get -n 100000 -c 10 --latency 2>&1 \
    | grep -v '^WARNING: Could not fetch server CONFIG$' || true
  echo ""
  echo ">>> fast_kv"
  redis-benchmark -p "$FASTKV_PORT" -t get -n 100000 -c 10 --latency 2>&1 \
    | grep -v '^WARNING: Could not fetch server CONFIG$' || true
}

# ── main ───────────────────────────────────────────────────────────────────
echo "fast_kv vs Redis benchmark"
echo "Runs per test: $RUNS"
check_server "$REDIS_PORT" "Redis"
check_server "$FASTKV_PORT" "fast_kv"

case "${1:-all}" in
  quick)   run_quick ;;
  stress)  run_pipeline; run_payload; run_clients ;;
  latency) run_latency ;;
  all)
    run_quick
    run_pipeline
    run_payload
    run_clients
    run_latency
    ;;
  *)
    echo "Usage: $0 [quick|stress|latency|all]"
    exit 1
    ;;
esac

section "Done"
echo "Tips:"
echo "  - Similar numbers in 'Quick sanity' = client-bound (expected)."
echo "  - Look for divergence in pipelined, large-payload, and client-ramp tests."
echo "  - fast_kv: cargo run --release --features local-logs"
echo "  - Redis:   redis-cli CONFIG SET save '' && redis-cli CONFIG SET appendonly no"
