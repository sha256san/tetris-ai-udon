#!/bin/bash
# ==============================================================================
# Start Parallel Training as a Background Daemon (Immune to SSH Disconnect)
# Usage: ./start_training_daemon.sh [ITERS_PER_WORKER (default: 500)] [REPEAT_ROUNDS (default: 0 = endless)]
# Examples:
#   ./start_training_daemon.sh 100 5    # 100 iters x 4 workers, repeated 5 rounds
#   ./start_training_daemon.sh 500 10   # 500 iters x 4 workers, repeated 10 rounds
#   ./start_training_daemon.sh 100      # 100 iters x 4 workers, endless loop
# ==============================================================================

ITERS=${1:-500}
MAX_ROUNDS=${2:-0}
LOG_FILE="logs/training_runner.log"
PID_FILE=".training_daemon_pid"

# Optimize glibc memory management & CPU threads to prevent memory buildup
export RAYON_NUM_THREADS=2
export MALLOC_TRIM_THRESHOLD_=131072
export MALLOC_MMAP_THRESHOLD_=131072

mkdir -p logs checkpoints

# 1. Check if already running
if [ -f "$PID_FILE" ]; then
    old_pid=$(cat "$PID_FILE")
    if [ -n "$old_pid" ] && kill -0 "$old_pid" 2>/dev/null; then
        echo "================================================================="
        echo "  ⚠️ Training daemon is already running (PID: $old_pid)."
        echo "  - View live progress: ./show_training_logs.sh"
        echo "  - Stop daemon:        ./stop_parallel_training.sh"
        echo "================================================================="
        exit 0
    fi
fi

# 2. Build release binary first
echo "[Build] Compiling release binary with optimized GPU/HIP shaders..."
cargo build --release || { echo "[Error] Build failed!"; exit 1; }

echo ""
echo "================================================================="
echo "  🚀 Starting Tetris AI Parallel Training Daemon"
echo "  Workers: 2 parallel instances"
echo "  Iterations per Worker per Round: $ITERS"
if [ "$MAX_ROUNDS" -gt 0 ]; then
echo "  Repeat Count: $MAX_ROUNDS rounds (Total Games: $((2 * ITERS * MAX_ROUNDS)))"
else
echo "  Repeat Count: Endless Loop (until ./stop_parallel_training.sh)"
fi
echo "  Auto Memory Cleanup: 100% process heap freed after each round"
echo "  SSH Disconnect Protection: Active (nohup + SIGHUP ignored)"
echo "================================================================="

# Start runner in background with nohup and disown
nohup ./run_parallel_training.sh "$ITERS" "$MAX_ROUNDS" > "$LOG_FILE" 2>&1 &
DAEMON_PID=$!
echo "$DAEMON_PID" > "$PID_FILE"
disown $DAEMON_PID 2>/dev/null

echo "  ✅ Daemon started in background (Master PID: $DAEMON_PID)"
echo "  📄 Main Runner Log: $LOG_FILE"
echo ""
echo "  💡 Commands:"
echo "     - View live progress: ./show_training_logs.sh"
echo "     - Stop training:      ./stop_parallel_training.sh"
echo "================================================================="
