#!/bin/bash
# ==============================================================================
# View Realtime Training Daemon Progress
# Usage: ./show_training_logs.sh
# ==============================================================================

LOG_FILE="logs/training_runner.log"

if [ ! -f "$LOG_FILE" ]; then
    echo "[Info] Log file $LOG_FILE not found. Is training running?"
    echo "       Start training with: ./start_training_daemon.sh"
    exit 1
fi

echo "================================================================="
echo "  📊 Streaming Tetris AI Training Logs (Press Ctrl+C to exit viewer)"
echo "  (Note: Exiting this viewer will NOT stop the background training)"
echo "================================================================="
echo ""

tail -f -n 40 "$LOG_FILE"
