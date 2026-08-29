#!/bin/bash
# ==============================================================================
# 4-Worker Parallel 10,000 Iterations Training Batch Runner
# Usage: ./run_parallel_training_10000.sh [OPTIONAL: ITERS_PER_WORKER (default: 10000)]
# ==============================================================================

set -m # Enable job control

ITERS=${1:-10000}
WORKERS=4
ROUND=1

mkdir -p logs checkpoints

echo "================================================================="
echo "       TETRIS AI 4-WORKER PARALLEL TUNING BATCH RUNNER"
echo "  Workers: $WORKERS parallel instances"
echo "  Iterations per Worker per Round: $ITERS"
echo "  Auto Best Model Sync: Yes (Highest Fitness -> model.json)"
echo "================================================================="

# Trap Ctrl+C (SIGINT) to kill all background workers cleanly
cleanup() {
    echo -e "\n\n[Interrupt received] Terminating all background workers..."
    kill -- -$$ 2>/dev/null
    exit 0
}
trap cleanup SIGINT SIGTERM

# 1. Build release binary first
echo "[Build] Compiling release binary with optimized GPU/HIP shaders..."
cargo build --release || { echo "[Error] Build failed!"; exit 1; }

while true; do
    echo ""
    echo "================================================================="
    echo "  🚀 Starting Round #$ROUND ($WORKERS Workers x $ITERS Iterations)"
    echo "  Time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "================================================================="

    PIDS=()
    for ((i=1; i<=WORKERS; i++)); do
        echo "  [Worker #$i] Launching 10,000 iters in background (log -> logs/worker_$i.log)..."
        ./target/release/tetris_ai \
            --tune-tspin "$ITERS" \
            --model-in model.json \
            --model-out "checkpoints/worker_${i}_best.json" \
            --worker "$i" > "logs/worker_$i.log" 2>&1 &
        PIDS+=($!)
    done

    echo ""
    echo "  ⚡ All $WORKERS Workers running in parallel (PIDs: ${PIDS[*]})"
    echo "  Monitoring progress... (Press Ctrl+C anytime to stop)"
    echo ""

    # Live progress loop
    while true; do
        sleep 5
        running_count=0
        for pid in "${PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                ((running_count++))
            fi
        done

        if [ "$running_count" -eq 0 ]; then
            break
        fi

        # Print quick one-line status summary from worker logs
        status_line="[Progress]"
        for ((i=1; i<=WORKERS; i++)); do
            latest=$(grep "Iteration" "logs/worker_$i.log" 2>/dev/null | tail -n 1 | awk -F'|' '{print $1 " | " $2}')
            if [ -n "$latest" ]; then
                status_line="$status_line [W#$i: $(echo $latest | xargs)]"
            else
                status_line="$status_line [W#$i: Starting...]"
            fi
        done
        printf "\r%-100s" "$status_line"
    done
    printf "\n"

    echo ""
    echo "[Complete] All $WORKERS workers finished Round #$ROUND ($((WORKERS * ITERS)) total iterations)!"

    # Evaluate best model and promote to model.json
    python3 scripts/merge_best_worker.py

    echo "================================================================="
    echo "  ✅ Round #$ROUND finished successfully!"
    echo "  model.json updated with best weights."
    echo "  Proceeding to Round #$((ROUND + 1)) in 3 seconds..."
    echo "================================================================="
    sleep 3
    ((ROUND++))
done
