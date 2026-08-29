#!/usr/bin/env python3
"""
Merge and promote the best worker model among 4 parallel workers to model.json
"""

import json
import os
import sys
from datetime import datetime

CHECKPOINT_DIR = "checkpoints"
MODEL_PATH = "model.json"
WORKER_COUNT = 4

def load_worker_result(worker_id: int):
    # Try latest checkpoint from worker
    vram_ckpt_path = os.path.join(CHECKPOINT_DIR, f"worker_{worker_id}_vram_checkpoint.json")
    model_out_path = os.path.join(CHECKPOINT_DIR, f"worker_{worker_id}_best.json")
    
    fitness = -1e9
    tsd = 0.0
    tst = 0.0
    tss = 0.0
    lines = 0.0
    weights = None

    if os.path.exists(vram_ckpt_path):
        try:
            with open(vram_ckpt_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                fitness = data.get("fitness", -1e9)
                tsd = data.get("avg_tsd", 0.0)
                tst = data.get("avg_tst", 0.0)
                tss = data.get("avg_tss", 0.0)
                lines = data.get("avg_lines", 0.0)
                weights = data.get("weights_from_vram", [])
        except Exception as e:
            print(f"[Warning] Failed to read {vram_ckpt_path}: {e}")

    if not weights and os.path.exists(model_out_path):
        try:
            with open(model_out_path, "r", encoding="utf-8") as f:
                data = json.load(f)
                weights = data.get("weights", [])
        except Exception as e:
            print(f"[Warning] Failed to read {model_out_path}: {e}")

    return {
        "worker_id": worker_id,
        "fitness": fitness,
        "avg_tsd": tsd,
        "avg_tst": tst,
        "avg_tss": tss,
        "avg_lines": lines,
        "weights": weights
    }

def main():
    print("\n" + "=" * 70)
    print(f"  [4-Worker Parallel Tuning Round Evaluation : {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}]")
    print("=" * 70)

    workers = []
    for wid in range(1, WORKER_COUNT + 1):
        info = load_worker_result(wid)
        workers.append(info)

    # Sort by fitness descending
    valid_workers = [w for w in workers if w["weights"] and len(w["weights"]) == 20]
    if not valid_workers:
        print("[Error] No valid worker models found in checkpoints/!")
        sys.exit(1)

    winner = max(valid_workers, key=lambda w: w["fitness"])

    print(f"{'Worker':<10} | {'Fitness':<12} | {'Avg TSD':<9} | {'Avg TST':<9} | {'Avg TSS':<9} | {'Avg Lines':<10} | Status")
    print("-" * 75)
    for w in workers:
        is_win = (w["worker_id"] == winner["worker_id"])
        win_mark = "👑 1st (WINNER -> model.json)" if is_win else "Completed"
        fit_str = f"{w['fitness']:.1f}" if w['fitness'] > -1e8 else "N/A"
        print(f"Worker #{w['worker_id']:<3} | {fit_str:<12} | {w['avg_tsd']:<9.2f} | {w['avg_tst']:<9.2f} | {w['avg_tss']:<9.2f} | {w['avg_lines']:<10.1f} | {win_mark}")
    print("=" * 75)

    # Update model.json
    model_data = {
        "weights": winner["weights"],
        "is_nonlinear": True,
        "backend": "Auto"
    }
    with open(MODEL_PATH, "w", encoding="utf-8") as f:
        json.dump(model_data, f, indent=2)

    print(f"\n[Success] 👑 Worker #{winner['worker_id']} (Fitness: {winner['fitness']:.1f}) promoted to {MODEL_PATH}!\n")

if __name__ == "__main__":
    main()
