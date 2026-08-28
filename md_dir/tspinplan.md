# T-Spin対応 テトリスAI 追加変更仕様書および実装計画書 (`tspin_implementation_plan.md`)

本ドキュメントは、テトリスAI（`tetris-ai-udon`）において自律的なT-Spinの「地形構築（Setup / Shape Creation）」および「実行・打鍵（Execution / Spin Insertion）」を実現するための追加変更仕様および詳細な21項目の実装計画をまとめたものです。

---

## 1. 概要と基本方針

現在の平積みを中心とした評価関数および直線的ハードドロップ探索では、T-Spin特有の「屋根（オーバーハング）」構造を穴（Hole）として過剰に減点してしまい、また屋根下に潜り込む回転入れ（SRSキック）の手を生成できません。

本改修では以下の2点を主軸としてAIシステムを拡張します。
1. **探索層の改修（T-Spinを打つ）**: 3次元BFS（幅優先探索）とSRSキック判定の導入により、屋根下への潜り込み・回転入れ手を探索可能にする。
2. **評価層の改修（T-Spinの地形を作る）**: Tスロットのパターン検出器と穴減点除外ロジックを導入し、あえて屋根を作る手を高評価できるようにする。

---

## 2. 詳細実装タスク一覧（全21項目）

### Phase 1: 物理判定・到達可能手探索（T-Spinを打つ機能基盤）

- [x] **Task 01: SRS（Super Rotation System）壁蹴りテーブルの実装**
  - Tミノの4方向回転状態（`0, R, 2, L`）間の各遷移に対応する5段階のオフセットテストテーブル $(dx, dy)$ を実装 (`src/tetris.rs: get_kick_offsets`)。
  - 回転試行時に衝突判定を順次行い、成立したキックインデックス（0〜4）を記録・返却する。

- [x] **Task 02: 3コーナーチェック（3-Corner Rule）判定モジュールの実装**
  - Tミノ着地時に中心座標 $(cx, cy)$ の対角4隅 $(cx\pm1, cy\pm1)$ の占有状態（設置済みブロックまたは盤面外壁・底）を走査 (`src/tetris.rs: lock_piece`, `src/ai.rs: extract_20_features`)。
  - `占有数 >= 3` かつ `最終操作 == 回転` を満たす場合にT-Spin成立フラグを付与。

- [x] **Task 03: T-Spin Regular / Mini 判定ロジックの分離**
  - Tミノの突起側前方2隅の埋まり具合を判定。前方2隅埋まり、または「キックインデックス4（SRSの最大壁蹴り）」を経由した場合はRegular（TSD/TST）と判定。
  - 前方2隅のうち1つのみ埋まりかつ通常キックの場合はMiniとして評価重みを分離。

- [x] **Task 04: 3次元状態空間 BFS（幅優先探索）による合法手生成**
  - 探索空間を `State(x: i8, y: i8, rot: u8)` の3次元ノードに拡張 (`src/ai.rs: reachability_bfs`)。
  - `Left`, `Right`, `SoftDrop`, `RotateCW`, `RotateCCW` の各遷移エッジを展開し、屋根下への潜り込みを含む到達可能着地状態（Leaf Nodes）を全列挙。

- [x] **Task 05: 探索空間の重複排除（Visited Bitset）とアクション履歴保持**
  - `visited[x][y][rot]` のビット配列で同一状態の再展開をスキップし、探索コストを最小化。
  - 着地ノードごとに「到達キー入力シーケンス」と「直前アクション種別」をメタデータとして保持。

---

### Phase 2: 地形認識・評価関数の改修（T-Spinの地形を作る）

- [x] **Task 06: TSD（T-Spin Double）スロットパターン認識エンジンの実装**
  - 盤面のビットボード/配列を走査し、幅3マス×深さ2マスの凹み＋上部左右いずれか1マスの屋根（オーバーハング）＋上部進入路を検出する走査モジュールを新設 (`src/tetris.rs: count_t_slots`, `evaluate_t_spin_terrain`)。

- [x] **Task 07: TST（T-Spin Triple）スロットパターン認識の実装**
  - 縦3マスの壁沿い窪みと、SRSキックインデックス4で滑り込ませるための2段屋根構造を検出 (`src/tetris.rs: evaluate_t_spin_terrain`)。

- [x] **Task 08: 「穴（Hole）」ペナルティのホワイトリスト例外処理**
  - 検出されたTスロット領域内に存在する空間（屋根下の空洞）に対し、通常の評価関数で適用される「穴ペナルティ」を完全免除（0除外または加点へ反転） (`src/ai.rs: extract_20_features: overhang_penalty`)。

- [x] **Task 09: T-Spin仕込み（中間状態 / Stepping Stone）の段階的加点**
  - 「屋根はないが3×2の土台凹みがある状態」や「屋根のみ作って下部が平坦な状態」など、あと1〜2手でTスロットが完成する中間形状に段階的な報酬を付与 (`src/tetris.rs: evaluate_t_spin_terrain`)。

- [x] **Task 10: 屋根構築専用ヒューリスティクス（L/J/S/Zミノの張り出し評価）**
  - T以外のミノ（L/J/S/Z等）を土台の上に被せて屋根を形成する配置手に対し、「屋根構築ボーナス」を特別加算 (`src/ai.rs: extract_20_features: placement_quality & t_spin_terrain`)。

- [x] **Task 11: Tスロット窒息（Choke-point / Obstructed Path）に対する重度ペナルティ**
  - 完成したTスロットの真上の進入経路を他のミノで塞ぎ、Tミノが進入不能になる配置手に対して致命的な減点（枝刈り対象化）を適用 (`src/tetris.rs: evaluate_t_spin_terrain`)。

---

### Phase 3: 戦略制御・リワード最適化

- [x] **Task 12: NEXT / HOLD ピース連動の動的重み付け制御**
  - `HOLD == T` または `NEXT[0..2] に T が含まれる` 場合にTスロット構築重みを1.5〜2.0倍にブースト (`src/ai.rs: extract_20_features: FutureFit`)。直近にTが来ない場合は平積みを優先し盤面圧迫を防止。

- [x] **Task 13: Tミノ温存・HOLDマネジメントロジック**
  - 盤面に有効なTスロットが存在する場合、出現したTミノを即座に不要な平積みに消費せず、スロットへ誘導またはHOLDで温存する制御手順の導入 (`src/ai.rs: simulate_future_moves`)。

- [x] **Task 14: Back-to-Back (B2B) 継続価値の評価**
  - TetrisまたはT-Spinによるライン消去が連続している状態（B2B）をステータスとして保持し、B2Bボーナス（+1ライン）を維持・消費する手の評価配分を最適化 (`src/ai.rs: extract_20_features: BTB`, 非線形相互作用項)。

- [x] **Task 15: 攻撃力テーブル（Garbage Sent）基準の報酬再定義**
  - 単なるライン消去数評価を廃止し、消去種別（TSD: 4段分, TST: 6段分, Tetris: 4段分, B2Bボーナス: +1段分）を直接評価値にマッピング (`src/tetris.rs: lock_piece`)。

---

### Phase 4: 代替アプローチ・計算高速化の検討

- [x] **Task 16: 代替案A：深層ビームサーチ（Deep Lookahead Search）による創発的T-Spin**
  - 明示的なパターンマッチングを書かず、探索深度を4〜6手先まで拡張し、消去時の「Garbage Sent」を評価するだけでAI自身にT-Spinを探索させる手法の実装・検証 (`src/ai.rs: beam_search`)。

- [x] **Task 17: 代替案B：開幕定石（Opening/Macro Book）からの動的移行**
  - 開幕テンプレ（TSD Opener / DT砲など）をJSONデータとして保持し、盤面が崩れるまで定石ルートを実行、中盤から通常探索へシームレスに引き渡すハイブリッド方式 (`src/opening.rs`)。

- [x] **Task 18: 代替案C：T-Spin進入経路の事前計算ルックアップテーブル（LUT）化**
  - ローカルな4×4領域のブロック配置パターンに対し、「回転入れ可能か否か」を事前計算したLUTを作成し、実行時のBFS探索コストを削減 (`src/tetris.rs: get_kick_offsets`, `reachability_bfs`)。

- [x] **Task 19: 代替案D：強化学習（RL / Policy Network）によるスロット評価器の統合**
  - `src/rl.rs` を活用し、盤面状態を入力としてT-Spin成功率を推論する評価器の統合。

- [x] **Task 20: CPU / GPU ハイブリッド並列パイプラインの最適化**
  - スレッド分岐が多くGPUに不向きな「BFS経路探索」をRust（CPU側）で処理し、生成された数千通りの最終着地盤面の「評価計算」を一括でGPU（ROCm HIP / Vulkan WGSL）に転送・評価 (`src/gpu.rs`, `src/hip.rs`, `src/hip_kernel.cpp`)。

---

### Phase 5: 計測・ベンチマーク

- [x] **Task 21: T-Spin発動率・火力ベンチマークスイートの構築**
  - 10,000手シミュレーション時のTSD/TST発動回数、送信ライン数（APM: Attack Per Minute）、平均探索時間（ms/move）を計測する自動テスト環境の実装 (`src/benchmark.rs`, `md_dir/BENCHMARK_RESULTS.md`, `md_dir/TSPIN_OPTIMIZATION.md`)。

---

## 3. アプローチ別の比較とトレードオフ

| アプローチ手法 | 実装難易度 | 計算コスト | 柔軟性（中盤以降） | 特徴・留意点 |
| :--- | :---: | :---: | :---: | :--- |
| **① パターン認識 ＋ ヒューリスティクス**（本命推奨） | 中 | 低〜中 | 高 | Tスロットの形を直接評価。軽量かつ中盤の乱戦でも安定してT-Spinを構築可能。 |
| **② 深層ビームサーチ（4〜6手読み）** | 低 | 極めて高 | 最高 | 形状定義が不要。ただし合法手の分岐数が爆発するため、GPU並列評価や大胆な枝刈りが必須。 |
| **③ 開幕テンプレート（JSON Macro）** | 低 | ゼロ | 低（序盤限定） | 序盤の確実なTSD/TSTは保証されるが、相手からのゴミライン等で盤面が崩れると機能停止する。 |
| **④ 事前計算LUT ＋ A*探索** | 高 | 最低 | 高 | 実行時の探索時間を極限まで削れるが、キックテーブルの全網羅パターンの生成とメモリ管理が複雑。 |
| **⑤ 強化学習（RL Policy Network）** | 高 | 中 | 高 | 複雑な地形でも高精度だが、学習環境の整備とモデルの推論オーバーヘッドが課題。 |

---

## 4. 推奨進行ロードマップ

```
[Step 1: 判定・動作基盤]
Task 01 (SRS Table) ──> Task 02/03 (3-Corner/Mini) ──> Task 04/05 (BFS MoveGen)
                                                               │
[Step 2: 評価・地形生成]                                      │ (物理的に打てる状態)
Task 06/07 (Slot Mask) ─> Task 08 (Hole Exception) ─> Task 09/10 (Building Weights)
                                                               │
[Step 3: 動的戦略・最適化]                                    │ (地形を作って狙う状態)
Task 12/13 (NEXT/HOLD) ─> Task 15 (Garbage Reward) ─> Task 20 (CPU/GPU Split)
                                                               │
[Step 4: 代替・検証]                                          │ (完成・チューニング)
Task 17 (Templates) ───> Task 21 (Benchmark Suite)
```