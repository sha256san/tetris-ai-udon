# 10. AI評価関数・探索・状態表現・学習アーキテクチャ (AI Architecture)

## 1. 20次元ハイブリッド評価関数
$$\text{Score} = \sum_{i=0}^{19} w_i \cdot x_i + \text{NonlinearBonuses} + \text{Heuristics}$$

### 特徴量一覧 ($x_0 \sim x_{19}$)
- $x_0$: T-Spin (TSD: 1.0, TST: 1.2, TSS: 0.6, Mini: 0.2, 空打ち: 0.0)
- $x_1$: T-Spin Terrain (STSD, 階段ドネイト, 3〜8列目単一穴品質)
- $x_2$: Hole Penalty (埋まった穴数 + 穴上ブロック数)
- $x_3$: Hole Spread Penalty (穴の空間的分散度)
- $x_4$: Placement Quality (着地高さ・屋根構築・ドネイト品質)
- $x_5$: Tetris (4ライン消去)
- $x_6 \sim x_8$: Pure Single / Double / Triple Penalty (無駄消し抑制)
- $x_9$: REN (コンボ継続)
- $x_{10}$: BTB (Back-to-Back状態)
- $x_{11} \sim x_{12}$: Max / Mean Combo
- $x_{13}$: Perfect Clear
- $x_{14}$: Height Penalty (総標高)
- $x_{15}$: Max Height Penalty (最高列標高)
- $x_{16}$: Bumpiness Penalty (表面凹凸度 + **中央山型集中凸度**)
- $x_{17}$: Well Quality (ガウシアン最適深さ4, **両端同時空き時は0.05へ急落**)
- $x_{18}$: Overhang Penalty (不純なオーバーハング)
- $x_{19}$: Future Fit (NEXT・HoldT/HoldI温存シナジー & WasteTペナルティ)

---

## 2. 探索アルゴリズム
- **3D BFS 到達可能性探索 (`search_reachable_landings`)**:
  - SRSウォールキック、ソフトドロップ、I-Spin/JL-Spin/SZ-Spinを全展開し、物理操作経路（`Vec<MoveAction>`）を完全追跡。
- **GPU並列ビームサーチ (`beam_search`)**:
  - ROCm HIP & Vulkan Compute Shader による高速バッチ評価（毎秒1,500万手以上）。
  - Depth 3〜5, Beam Width 30〜50。

---

## 3. 進化学習 (CMA-ES / GA) & VRAM同期
- 各イテレーションでGPU VRAMメモリと完全同期し、重みチェックポイント（`checkpoints/vram_model_iter_*.json`）を逐次保存。
