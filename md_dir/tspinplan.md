# T-Spin対応 テトリスAI 詳細追加変更仕様書および実装計画書 (`md_dir/tspinplan.md`)

本ドキュメントは、テトリスAI（`tetris-ai-udon`）において、世界トップクラスのテトリスAI実装である **HoikoCode**（[ultimacrown/HoikoCode20230120](https://github.com/ultimacrown/HoikoCode20230120)）のアーキテクチャ・評価理論を大いに参考とし、自律的なT-Spinの「地形構築（Setup / Shape Creation）」、「ドネーション（Donation / Roof）」、「回転入れ打鍵（Execution / Spin Insertion）」および「TD砲（Triple-Double）連携」を実現するための追加変更仕様および詳細実装計画をまとめたものです。

---

## 1. HoikoCode（ultimacrown）のT-Spin設計思想と本AIへの統合

HoikoCodeの評価システム（`EvaluateAI`）および手生成システム（`ExpandAI`）を分析すると、T-Spinを単なる「回転後の消去」としてではなく、**地形形成から発火、後処理に至るライフサイクル全体**として緻密にモデル化しています。

### 1.1 HoikoCodeに学ぶ主要評価概念（W列挙体）
HoikoCodeの評価重み定義（`EvaluateAI::W`）に基づく主要T-Spin指標：

1. **スロット認識と進入可否の分離**:
   - `TsdHole`: T-Spin Double用の3×2窪みと土台形状の認識。
   - `TsdSpinable`: SRSキックを用いてTミノが実際に物理的に潜り込み・回転入れ可能かどうかの判定。
   - `TsdClearable`: TSD発火後に生じる残余地形が平坦かつクリーンであるかの判定。
   - `TsdOffensive`: 発火時の攻撃力（4ライン＋B2B）の直接加点。
2. **高難度T-Spin（TST & TD Cannon）の認識**:
   - `TstHole` / `TstSpinable` / `TstClearable` / `TstHint` / `TstOffensive`: 縦3マスの壁際スロットとSRSキックインデックス4によるT-Spin Tripleの認識。
   - `TDHole` / `TDHint`: **TD砲（TSTからTSDへの連続遷移構造）** の土台および誘導ヒント。
3. **屋根（Roof）とドネーション（Donate）の精緻な分類**:
   - `DonateCover` / `Cover`: 下部に穴を残さず、または後で回収可能な意図的T-Spin屋根（ドネーション）。
   - `Roof` vs `BadRoof`: T-Spin発火に寄与する有効な屋根と、単に盤面を窒息させる有害な浮きブロックの峻別。
   - `Pierce` / `Anabara`: 貫通性のある穴と、T-Spin地形下の許容空洞の分離。
4. **ミノ資源管理（Resource Management）**:
   - `HoldT` / `WasteT`: 有効なTスロットが存在する際に、Tミノを通常消去に無駄遣い（`WasteT`）することを厳罰化し、HOLD温存（`HoldT`）を推奨。
   - `HoldI` / `WasteI`: テトリス（4列消去）用のIミノ資源管理。
5. **評価値の伝播と信頼度（Nexus & TrustRate）**:
   - `Nexus`: 手の直接評価（攻撃力・消去）と中長期盤面評価（地形品質）をブレンドする係数。
   - `TrustRate`: ネクストキューの可視範囲を超える深層探索ノードに対する割引減衰率。

---

## 2. 詳細実装タスク一覧（全5フェーズ・25項目）

### Phase 1: 物理判定・3次元手生成（T-Spinを打つ基盤）

- [x] **Task 01: SRS（Super Rotation System）壁蹴りテーブルの実装**
  - Tミノの4方向回転遷移（`0->R, R->0, R->2, 2->R, 2->L, L->2, L->0, 0->L`）に対応する5段階のオフセットテーブル $(dx, dy)$ を完全実装 (`src/tetris.rs: get_kick_offsets`)。
  - キック成功時にインデックス（0〜4）を記録。

- [x] **Task 02: 3コーナーチェック（3-Corner Rule）判定モジュールの実装**
  - Tミノ着地時に中心 $(cx, cy)$ の対角4隅の占有状態（ブロックまたは壁・底）を走査 (`src/tetris.rs: lock_piece`, `src/ai.rs: extract_20_features`)。
  - `占有数 >= 3` かつ `直前操作 == 回転` でT-Spin判定。

- [x] **Task 03: T-Spin Regular / Mini 判定ロジックの分離**
  - Tミノの突起前方2隅の埋まり、またはキックインデックス4経由時をRegular（TSD/TST）とし、前方1隅埋まり通常キック時をMiniとしてスコア・重みを分離。

- [x] **Task 04: 3次元状態空間 BFS（幅優先探索）による合法手生成**
  - `State(x: i8, y: i8, rot: u8)` の3次元ノード空間を展開 (`src/ai.rs: reachability_bfs`)。
  - `Left`, `Right`, `SoftDrop`, `RotateCW`, `RotateCCW` の遷移エッジを展開し、屋根下への潜り込み（Tuck）を含む着地ノードを全列挙。

- [x] **Task 05: 探索空間の重複排除（Visited Bitset）とアクション履歴保持**
  - `visited[x][y][rot]` のビット配列で同一状態の再展開をスキップし、探索コストを最小化。

---

### Phase 2: HoikoCode準拠 地形認識・評価関数の改修（T-Spinの地形を作る）

- [x] **Task 06: TSD（T-Spin Double）スロットパターン認識エンジンの実装**
  - 盤面を走査し、幅3マス×深さ2マスの凹み＋上部左右いずれか1マスの屋根（オーバーハング）＋上部進入路を検出する走査モジュールを実装 (`src/tetris.rs: count_t_slots`, `evaluate_t_spin_terrain`)。

- [x] **Task 07: TST（T-Spin Triple）スロットパターン認識の実装**
  - 縦3マスの壁沿い窪みと、SRSキックインデックス4で滑り込ませるための2段屋根構造を検出 (`src/tetris.rs: evaluate_t_spin_terrain`)。

- [ ] **Task 08: HoikoCode流 TD砲（Triple-Double Cannon）複合パターンの検出**
  - TSTの2段屋根の上にさらにTSDスロットが重なる「TD砲（Trinity / DT Cannon）」の土台形状を走査し、極大ボーナスを付与。

- [x] **Task 09: 「穴（Hole）」ペナルティのホワイトリスト例外処理（DonateCover）**
  - Tスロット領域内に存在する空間（屋根下の空洞）に対し、通常の「穴ペナルティ」を完全免除し、有効なドネーション（DonateCover）として加点評価 (`src/ai.rs: extract_20_features: overhang_penalty`)。

- [x] **Task 10: T-Spin仕込み（中間状態 / Stepping Stone）の段階的加点**
  - 「屋根はないが3×2の土台凹みがある状態」や「屋根のみ作って下部が平坦な状態」など、あと1〜2手でTスロットが完成する中間形状に段階的な報酬を付与 (`src/tetris.rs: evaluate_t_spin_terrain`)。

- [x] **Task 11: 屋根構築専用ヒューリスティクス（L/J/S/Zミノの張り出し評価）**
  - T以外のミノ（L/J/S/Z等）を土台の上に被せて屋根を形成する配置手に対し、「屋根構築ボーナス」を特別加算 (`src/ai.rs: extract_20_features: placement_quality & t_spin_terrain`)。

- [x] **Task 12: Tスロット窒息（Choke-point / BadRoof）に対する重度ペナルティ**
  - 完成したTスロットの進入経路を不要なブロックで塞ぐ手に対して致命的な減点を適用 (`src/tetris.rs: evaluate_t_spin_terrain`)。

---

### Phase 3: 戦略制御・リソースマネジメント（HoikoCode流ミノ運用）

- [x] **Task 13: NEXT / HOLD ピース連動の動的重み付け制御**
  - `HOLD == T` または `NEXT[0..2] に T が含まれる` 場合にTスロット構築重みをブースト (`src/ai.rs: extract_20_features: FutureFit`)。

- [ ] **Task 14: HoikoCode流 WasteT（Tミノ無駄遣い）防止・HOLD温存ロジック**
  - 盤面に有効なTスロットが存在するにもかかわらず、Tミノを通常平積みで消費する手に重いペナルティ（`WasteT`）を課し、HOLD温存（`HoldT`）を最優先誘導。

- [x] **Task 15: Back-to-Back (B2B) 継続価値の評価**
  - TetrisまたはT-Spinによるライン消去が連続している状態（B2B）をステータスとして保持し、B2Bボーナス（+1ライン）を維持・消費する手の評価配分を最適化 (`src/ai.rs: extract_20_features: BTB`, GPU非線形相互作用項)。

- [x] **Task 16: 攻撃力テーブル（Garbage Sent）基準の報酬再定義**
  - 消去種別（TSD: 4段分, TST: 6段分, Tetris: 4段分, B2Bボーナス: +1段分）を直接評価値にマッピング (`src/tetris.rs: lock_piece`)。

- [ ] **Task 17: Nexus（評価ブレンド）とTrustRate（深層信頼度減衰）の実装**
  - 手の直接攻撃スコアと盤面地形スコアを `Nexus` 係数で統合し、可視ネクストを超える深さのノードに `TrustRate` 減衰率を乗算。

---

### Phase 4: GPU / CPU 並列ハイブリッド探索と最適化

- [x] **Task 18: 深層ビームサーチ（Depth 1〜5）による創発的T-Spin探索**
  - 探索深度を最大5手先まで拡張し、消去時の攻撃力と地形価値から最善のT-Spinシーケンスを創発的に選択 (`src/ai.rs: beam_search`)。

- [x] **Task 19: 開幕定石（Opening / Macro Book）からの動的移行**
  - 開幕テンプレ（TSD Opener, TKI 3, DT Cannon等）をJSONデータとして保持し、盤面が崩れるまで定石ルートを実行、中盤から通常探索へシームレスに引き渡すハイブリッド方式 (`src/opening.rs`)。

- [x] **Task 20: CPU（BFS手生成）＋ GPU（ROCm HIP / Vulkan 高次多項式評価）のハイブリッドパイプライン**
  - CPU側で分岐の多いBFS到達可能性探索を行い、数千の着地候補盤面をGPU（AMD ROCm HIP / Vulkan WGSL）で並列一括評価 (`src/gpu.rs`, `src/hip.rs`, `src/hip_kernel.cpp`)。

- [x] **Task 21: GPU VRAM（デバイスメモリ）直接同期 & チェックポイント永続化**
  - GPU VRAM上の重みバッファと直接同期し、`checkpoints/vram_model_iter_*.json` および `vram_weights_checkpoint.json` へリアルタイム永続化。

---

### Phase 5: 計測・ベンチマーク & チューニング

- [x] **Task 22: 100回 適応型進化戦略によるT-Spin特化パラメータ最適化**
  - 多シード自己対戦シミュレーションと適応型変異により、T-Spin発火力を最大化する重みベクトルを自動導出 (`src/tuning.rs`)。

- [x] **Task 23: T-Spin カテゴリ別（TSS / TSD / TST / Mini / 総計 / T-Slot形成）ベンチマーク**
  - 探索アルゴリズムごとの T-Spin 種類別実戦発火回数、T-Slot 形成回数、Tetris 消去数を正確に計測・集計する分析表を構築 (`src/benchmark.rs`, `md_dir/BENCHMARK_RESULTS.md`, `md_dir/TSPIN_OPTIMIZATION.md`)。

- [ ] **Task 24: APM（Attack Per Minute）および火力効率計測機能の追加**
  - 1分間あたりの送信ライン数（APM）およびミノ消費効率（Attack per Piece）を計測するベンチマーク指標の拡充。

- [ ] **Task 25: HoikoCode対戦シミュレーション検証**
  - 対戦モードにおけるゴミライン相殺、カウンターTSD、掘り（Downstacking）中のドネーションTSDの挙動検証。

---

## 3. HoikoCodeの主要手法と本AIの比較

| 項目 | HoikoCode (2023) | 本Tetris AI (`tetris-ai-udon`) | 導入・強化状況 |
| :--- | :--- | :--- | :--- |
| **手生成** | `ExpandBaseNode` + `ExpandDeriveNode` | 3次元 `reachability_bfs` (x, y, rot) | ✅ 実装済み（SRSキック・潜り込み対応） |
| **TSD評価** | `TsdHole`, `TsdSpinable`, `TsdClearable` | `extract_20_features` ($x_0, x_1, x_4, x_{18}$) | ✅ 実装済み（100回最適化済み重み） |
| **TST評価** | `TstHole`, `TstSpinable`, `TstClearable` | `evaluate_t_spin_terrain` + SRSキック4 | ✅ 実装済み |
| **TD砲評価** | `EvalDTCannon` / `TDHole` / `TDHint` | `evaluate_t_spin_terrain` | 🔄 Task 08 で専用パターン検出を強化予定 |
| **屋根・穴分類** | `DonateCover`, `BadRoof`, `Anabara` | T-Slot屋根減点免除 + Choke-point減点 | ✅ 実装済み |
| **ミノ資源管理** | `HoldT`, `WasteT`, `HoldI`, `WasteI` | $x_{19}$ (`FutureFit`) + Tミノ温存 | 🔄 Task 14 で `WasteT` 厳罰化を強化予定 |
| **評価統合** | `Nexus` (0〜100%) + `TrustRate` | 非線形多項式相互作用項 + ビームサーチ | 🔄 Task 17 で Nexus 形式のブレンド導入予定 |
| **アクセラレーション** | CPU マルチスレッド | **AMD ROCm 7.1 HIP ＋ Vulkan wgpu GPU** | 🚀 **GPU並列計算により高速化** |

---

## 4. 今後の進行ロードマップ

```
[Step 1: 物理判定・基本TSD/TST基盤] (完了)
Task 01〜05 (SRS / 3-Corner / BFS) ──> Task 06〜07, 09〜12 (TSD/TST/屋根) ──> Task 18, 20〜23 (GPU / 100回最適化)
                                                                                       │
[Step 2: HoikoCode流 高度T-Spin拡張] (次期実装)                                        │
Task 08 (TD砲 / DT Cannon検出) ───> Task 14 (WasteT/HoldT資源管理) ───> Task 17 (Nexus/TrustRate)
                                                                                       │
[Step 3: 対戦火力・APMベンチマーク拡充]                                                │
Task 24 (APM / 火力効率計測) ─────> Task 25 (対戦シミュレーション検証)
```