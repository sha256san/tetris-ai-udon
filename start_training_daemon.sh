#!/bin/bash
# ==============================================================================
# Start Parallel Training as a Background Daemon (Immune to SSH Disconnect)
# Usage: ./start_training_daemon.sh [OPTIONAL: ITERS_PER_WORKER (default: 500)]
# Examples:
#   ./start_training_daemon.sh       # 4 workers x 500 iters/round
#   ./start_training_daemon.sh 100   # 4 workers x 100 iters/round
# ==============================================================================

ITERS=${1:-500}
LOG_FILE="logs/training_runner.log"
PID_FILE=".training_daemon_pid"

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
echo "  Workers: 4 parallel instances"
echo "  Iterations per Worker per Round: $ITERS"
echo "  SSH Disconnect Protection: Active (nohup + SIGHUP ignored)"
echo "================================================================="

# Start runner in background with nohup and disown
nohup ./run_parallel_training.sh "$ITERS" > "$LOG_FILE" 2>&1 &
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
