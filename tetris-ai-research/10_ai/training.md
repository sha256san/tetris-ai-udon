# 10-D. 進化学習・強化学習・VRAM同期 (Training & Optimization)

## 1. 1000回適応型進化学習 (`--tune-tspin 1000`)
- **Fitness 関数**:
  $$\text{Fitness} = \text{LinesCleared} \times 10 + \text{TSD} \times 1500 + \text{TST} \times 2500 + \text{TSS} \times 400 + \text{B2B} \times 500 - \text{EmptyTSpin} \times 800$$
- **GPU VRAM同期**:
  - 各イテレーションでモデル重みをチェックポイント（`checkpoints/vram_model_iter_*.json`）にダンプし、GPUクラッシュや中断時も安全に再開可能。
