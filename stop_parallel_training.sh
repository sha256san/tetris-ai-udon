#!/bin/bash
# ==============================================================================
# Stop All Parallel Training Workers Script
# Usage: ./stop_parallel_training.sh
# ==============================================================================

echo "================================================================="
echo "       TETRIS AI: STOPPING ALL PARALLEL TRAINING WORKERS"
echo "================================================================="

PID_FILE=".training_pids"
DAEMON_PID_FILE=".training_daemon_pid"

# 1. Kill daemon if exists
if [ -f "$DAEMON_PID_FILE" ]; then
    dpid=$(cat "$DAEMON_PID_FILE")
    if [ -n "$dpid" ] && kill -0 "$dpid" 2>/dev/null; then
        echo "[Info] Terminating training daemon (PID: $dpid)..."
        kill "$dpid" 2>/dev/null
    fi
    rm -f "$DAEMON_PID_FILE"
fi

# 2. Kill from PID file if exists
if [ -f "$PID_FILE" ]; then
    echo "[Info] Reading worker PIDs from $PID_FILE..."
    while read -r pid; do
        if [ -n "$pid" ] && kill -0 "$pid" 2>/dev/null; then
            echo "  - Terminating PID: $pid"
            kill "$pid" 2>/dev/null
        fi
    done < "$PID_FILE"
    rm -f "$PID_FILE"
fi

# 3. Kill any remaining tetris_ai tuning processes and batch runners
echo "[Info] Ensuring all background workers and batch runners are stopped..."
pkill -f "run_parallel_training.sh" 2>/dev/null
pkill -f "run_parallel_training_100.sh" 2>/dev/null
pkill -f "run_parallel_training_10000.sh" 2>/dev/null
pkill -f "tetris_ai --tune-tspin" 2>/dev/null

sleep 1

# 4. Check remaining processes
rem=$(pgrep -f "tetris_ai --tune-tspin" | wc -l)
if [ "$rem" -gt 0 ]; then
    echo "[Warning] Force killing $rem remaining worker process(es)..."
    pkill -9 -f "tetris_ai --tune-tspin" 2>/dev/null
fi

echo ""
echo "[Preserve] Evaluating and preserving latest best model to model.json..."
python3 scripts/merge_best_worker.py 2>/dev/null

echo "================================================================="
echo "  ✅ All parallel training workers stopped successfully."
echo "  Latest best weights are preserved in model.json."
echo "================================================================="
