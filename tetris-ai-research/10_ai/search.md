# 10-B. 探索アルゴリズム詳細 (Search Algorithms)

## 1. 3D BFS Reachability Search
- 状態空間: $(x, y, \text{rotation}) \in [-3..12] \times [0..23] \times [0..3]$
- 操作アクション: MoveLeft, MoveRight, SoftDrop, RotateCW, RotateCCW
- 各着地について、操作履歴 `path: Vec<MoveAction>` を保持し、実戦でソフトドロップや横入れ・スピン入れを完全再現。

## 2. GPU Accelerated Beam Search
- 深さ $D=3 \sim 5$, ビーム幅 $K=30 \sim 50$
- 各階層で候補手（40手）を展開し、GPU（ROCm HIP / Vulkan wgpu）で1,000〜5,000手を一括並列評価。
