#!/bin/bash
# ==============================================================================
# 4-Worker Parallel 100 Iterations Training Batch Runner
# Usage: ./run_parallel_training_100.sh
# ==============================================================================

ITERS=${1:-100}
WORKERS=4
ROUND=1
PID_FILE=".training_pids"

# Optimize glibc memory management & CPU threads to prevent memory buildup
export RAYON_NUM_THREADS=2
export MALLOC_TRIM_THRESHOLD_=131072
export MALLOC_MMAP_THRESHOLD_=131072

mkdir -p logs checkpoints

echo "$$" > "$PID_FILE"

echo "================================================================="
echo "       TETRIS AI 4-WORKER PARALLEL TUNING BATCH RUNNER (100 ITERS)"
echo "  Workers: $WORKERS parallel instances"
echo "  Iterations per Worker per Round: $ITERS"
echo "  Total Games per Round: $((WORKERS * ITERS))"
echo "  Auto Best Model Sync: Yes (Highest Fitness -> model.json)"
echo "  Auto Loop across Rounds: Yes (Round 1 -> Round 2 -> Round 3 ...)"
echo "  Stop command: ./stop_parallel_training.sh or Ctrl+C"
echo "================================================================="

# Ignore SIGHUP explicitly so SSH disconnection will NOT kill the training loop
trap '' HUP

cleanup() {
    trap - SIGINT SIGTERM EXIT
    echo -e "\n\n[Interrupt received] Terminating all background workers..."
    if [ -f "$PID_FILE" ]; then
        while read -r pid; do
            if [ -n "$pid" ] && [ "$pid" != "$$" ]; then
                kill "$pid" 2>/dev/null
            fi
        done < "$PID_FILE"
        rm -f "$PID_FILE"
    fi
    pkill -P $$ 2>/dev/null
    exit 0
}
trap cleanup SIGINT SIGTERM

echo "[Build] Compiling release binary with optimized GPU/HIP shaders..."
cargo build --release || { echo "[Error] Build failed!"; rm -f "$PID_FILE"; exit 1; }

while true; do
    echo ""
    echo "================================================================="
    echo "  🚀 Starting Round #$ROUND ($WORKERS Workers x $ITERS Iterations)"
    echo "  Time: $(date '+%Y-%m-%d %H:%M:%S')"
    echo "================================================================="

    PIDS=()
    for ((i=1; i<=WORKERS; i++)); do
        echo "  [Worker #$i] Launching $ITERS iters in background (log -> logs/worker_$i.log)..."
        nohup ./target/release/tetris_ai \
            --tune-tspin "$ITERS" \
            --model-in model.json \
            --model-out "checkpoints/worker_${i}_best.json" \
            --worker "$i" > "logs/worker_$i.log" 2>&1 &
        pid=$!
        PIDS+=($pid)
        echo "$pid" >> "$PID_FILE"
        disown "$pid" 2>/dev/null
    done

    echo ""
    echo "  ⚡ All $WORKERS Workers running in parallel (PIDs: ${PIDS[*]})"
    echo "  Monitoring progress... (Press Ctrl+C or run ./stop_parallel_training.sh to stop)"
    echo ""

    while true; do
        sleep 2
        running_count=0
        for pid in "${PIDS[@]}"; do
            if kill -0 "$pid" 2>/dev/null; then
                ((running_count++))
            fi
        done

        if [ "$running_count" -eq 0 ]; then
            break
        fi

        status_line="[Progress]"
        for ((i=1; i<=WORKERS; i++)); do
            latest=$(grep "Iteration" "logs/worker_$i.log" 2>/dev/null | tail -n 1 | awk -F'|' '{print $1 " | " $2}')
            if [ -n "$latest" ]; then
                status_line="$status_line [W#$i: $(echo $latest | xargs)]"
            else
                status_line="$status_line [W#$i: Starting...]"
            fi
        done
        printf "\r%-120s" "$status_line"
    done
    printf "\n"

    echo ""
    echo "[Complete] All $WORKERS workers finished Round #$ROUND ($((WORKERS * ITERS)) total iterations)!"

    python3 scripts/merge_best_worker.py

    echo "================================================================="
    echo "  ✅ Round #$ROUND finished successfully!"
    echo "  model.json updated with best weights."
    echo "  🚀 Automatically proceeding to Round #$((ROUND + 1)) in 2 seconds..."
    echo "================================================================="
    sleep 2
    ((ROUND++))
done
