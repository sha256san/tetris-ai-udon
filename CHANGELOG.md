# 変更履歴 (CHANGELOG.md)

すべての変更内容は本ドキュメントに記録されます。

## [0.1.4] - 2026-08-30

### 追加 (Added)
- **4体同時 10,000回反復 並列進化学習バッチシステム (`run_parallel_training_10000.sh`, `scripts/merge_best_worker.py`)**:
  - 4つのAIワーカープロセスを完全並行（バックグラウンド）で起動し、各ワーカーが10,000イテレーションずつ探索を実行。
  - 各ラウンド終了時に全ワーカーのFitness・TSD/TST発火回数・VRAMチェックポイントを自動比較し、最高性能の重みを `model.json` に昇格させて次ラウンドへ自動継承する永続ループを実装。
- **実戦特化 操作高速化エンジン (`src/ai.rs`, `src/main.rs`)**:
  - `optimize_execution_path`: 上空から直線落下可能なオープン着地点に対して、無駄なソフトドロップを完全に排除し最短の `[Rotate, Shift, HardDrop]` 直通パスを生成。
  - スピン入れ（SRSキック）や潜り込みが必要な手についても、目標セル到達直後に末尾へ `MoveAction::HardDrop` を追加し、0.5秒の接地ロックディレイ待機時間を完全ゼロ化。
  - `MoveAction::HardDrop` 列挙子および自動プレイ描画ループ対応。
- **T-Spin Mini 後の BTB 継続検証 & 追跡ログ機能 (`src/tspin_recorder.rs`)**:
  - `TSpinEventRecord` に `btb_continued_after`, `btb_evaluation`, `next_heavy_attack` を追加。
  - Mini発火後の5手を自動走査し、BTBを維持したまま本命火力（Tetris や TSD/TST）を発火できたかを判定（`Successful Mini -> BTB Maintained` または `Wasted Mini`）。
- **T-Spin 直前1手（Turn -1: 屋根・仕込み手）のログ強調表示 (`src/tspin_recorder.rs`)**:
  - T-Spin発火直前の1手を視覚的な専用バナー `🔥🔥🔥 【T-SPIN SETUP MOVE: 1 TURN BEFORE TRIGGER (Turn X)】 🔥🔥🔥` で目立たせ、どのミノで屋根や土台をセットアップしたかを明確化。
- **T-Spin Mini 無駄打ちペナルティ (`src/config.rs`, `src/ai.rs`)**:
  - `WASTED_TSPIN_MINI_PENALTY: -30.0` を追加し、BTBに繋がらない単発Miniを評価関数側で抑制。
- **Web対戦サーバーの動的ポートフォールバック (`src/server.rs`)**:
  - ポート3000が使用中の場合でもパニックせず、自動的に3001〜3010へ切り替えて起動する堅牢なバインド処理を実装。

### テスト (Testing)
- 全32件の単体テスト（`cargo test`）が100%合格。
  - `test_hard_drop_preference_and_instant_lock`
  - `test_wasted_mini_suppression`
  - `test_setup_turn_highlighting`
  - `test_tspin_mini_btb_continuation_tracking`

---

## [0.1.3] - 2026-08-30

### 追加 (Added)
- **1,180項目 構造化テトリス知識ベース & リサーチ基盤 (`tetris-ai-research/`, `src/knowledge.rs`, `scripts/generate_tetris_research.py`)**:
  - `addplan3.md` 準拠の全16カテゴリ・1,180件の詳細知識をJSON（`knowledge.json`, `terrain_patterns.json`）およびMarkdownリファレンス文書（`01_rules/` 〜 `12_sources/`）として完全体系化。
  - Rustエンジン用知識管理モジュール `src/knowledge.rs` を追加し、`KnowledgeBase::load_from_default_path()` による高速検索をサポート。
- **実戦特化 地形評価 & 物理制約エンジン (`src/tetris.rs`, `src/ai.rs`, `src/config.rs`)**:
  - `calculate_center_convexity`: 盤面中央列（x=3..6）が高く盛り上がる富士山型地形を検知し、`CENTER_CONVEXITY_PENALTY (-40.0)` および特徴量 $x_{16}$ でペナルティ付与。
  - `detect_dual_side_wells`: x=0 と x=9 が同時に深さ2以上の穴になる状態を検知し、`DUAL_SIDE_WELL_PENALTY (-100.0)` を付与、特徴量 $x_{17}$ を 0.05 に急落させてIミノ枯渇死を防止。
  - `evaluate_t_slot_column_position`: 2〜9列目（特に3〜8列目推奨）の単一列穴（幅1マス）を最高評価（1.0）、発火後も盤面がフラットに保たれる手を優先。
  - `validate_wall_tst_orientation`: 左壁（x=0）および右壁（x=9）のTSTにおいて、空中浮遊を排除した**盤面内向き屋根構造**のみを有効認定（+150.0）。
  - `detect_kaidan_setup_patterns`: しゑひ式「階段のドネイト（Kaidan Setups）」を検知しボーナス（+45.0）を付与。
- **1000回適応型進化学習 (`--tune-tspin 1000`)**:
  - Fitness が `4270.0` $\rightarrow$ `6910.0` (+61.8%) へ大幅向上。

---

### 追加 (Added)
- **T-Spin特化 評価関数チューニングエンジン (`src/tuning.rs`)**:
  - `addplan.md` および `PLAN.md` 準拠の 20特徴量非線形評価関数に対し、適応型変異進化戦略による T-Spin（TSD / TST / TSS）および T-Slot 構築力に特化した最適化ループを実装。
  - CLI オプション `--tune-tspin` / `-t` およびメインメニュー `[3] T-spin 100-Iteration Optimization (VRAM Data Persistence)` を追加。
- **GPU VRAM（デバイスメモリ）データ直接同期 & 永続化機能 (`src/hip.rs`, `src/hip_kernel.cpp`)**:
  - `upload_weights_to_vram`: ホスト側の探索重みを GPU VRAM (`d_weights`) へ直接アップロード。
  - `readback_weights_from_vram`: GPU VRAM 上の重みバッファから直接リードバックして整合性を保証。
  - `get_vram_usage`: `hipMemGetInfo` を用いたリアルタイムな VRAM 使用量監視（Free / Total）。
  - `checkpoints/vram_model_iter_*.json` および `vram_weights_checkpoint.json` へ VRAM スナップショットを逐次出力・永続化。
- **詳細 T-Spin カテゴリ別（TSS / TSD / TST / Mini / 総計 / T-Slot形成）ベンチマーク (`src/benchmark.rs`, `md_dir/TSPIN_OPTIMIZATION.md`)**:
  - 探索アルゴリズムごとの T-Spin 種類別実戦発火回数、T-Slot 形成回数、Tetris 消去数を正確に計測・集計する分析表を追加。

### 変更 (Changed)
- `src/tetris.rs`:
  - `count_t_slots`: 全列（壁際・床際含む）の T-Slot 検出に対応し、実ブロックコーナー判定を厳格化。
  - `evaluate_t_spin_terrain`: T-Slot 完成形だけでなく、屋根候補付き窪みや基礎地形を正当に加点。
- `src/ai.rs`:
  - `extract_20_features`: 候補手直接の 3 コーナー判定による T-Spin 発火スコア算出、T-Slot 屋根のオーバーハング減点免除、T ミノ保持時のシナジー加点を追加。
  - `new_20_feature_default`: 100回最適化済みの高性能重みセット（$w_0=+88.07, w_1=+61.85$ 等）をデフォルトに反映。
- `src/main.rs`: 100回最適化、VRAM 同期表示、T-Spin 詳細内訳表の生成に対応。

---

## [0.1.1] - 2026-08-28

### 追加 (Added)
- **AMD ROCm (HIP) ネイティブコンピュートエンジン (`src/hip.rs`, `src/hip_kernel.cpp`)**:
  - AMD Radeon RX 9060 XT (`gfx1200`) の ROCm 7.1 / HIP ランタイムを直接呼び出すネイティブ C++/HIP カーネルを実装。
  - `build.rs` により `hipcc` を自動検出し、`libtetris_hip.a` および `libamdhip64` をリンクして GPU 上で直接バッチ評価を実行。
  - Vulkan (wgpu) と ROCm (HIP) のデュアル GPU バックエンド切り替え・自動フォールバックに対応。
- **20次元 非線形多項式ハイブリッド評価関数 (`md_dir/addplan.md` 準拠)**:
  - [`src/ai.rs`](file:///home/sha256san/tetris_ai/src/ai.rs): T-spin（Single/Double/Triple/Mini）、T-spin地形品質、穴数、穴深度・埋没穴、穴分散（列分散+マンハッタン距離）、配置品質、テトリス、Pure Single/Double/Triple、REN、BTB、Max/Mean Combo、PC、盤面総高さ、最大高さ、凹凸、ガウス型井戸品質、オーバーハング、将来適合度（Next/Hold）の 20 正規化特徴量を抽出 (`extract_20_features`)。
  - 二次交互作用項（$TSpin \times TSpinTerrain$, $Tetris \times WellQuality$, $Tetris \times BTB$ 等）、三次項、指数関数型高さペナルティ、累乗型穴ペナルティ、飽和型ボーナス（REN/BTB/Combo）を GPU（ROCm HIP / Vulkan WGSL）上で並列高速計算。
- **ROCm vs Vulkan 性能比較ベンチマーク (`md_dir/ROCM_VULKAN_BENCHMARK.md`)**:
  - マイクロベンチマーク（バッチサイズ $N=10〜5000$ のディスパッチ遅延・スループット）およびマクロベンチマーク（固定シード実戦探索プレイ）を実施し、詳細レポートを生成。
- **メインメニュー [8] アルゴリズム選択（ランキング順）の追加 (`src/main.rs`)**:
  - ベンチマーク検証結果（1位〜8位）に基づき、ソートされた探索アルゴリズム一覧から即座にアクティブなアルゴリズム構成（探索深度、ビーム幅、GPU/ROCm/Vulkan/CPUバックエンド）を選択・切り替え可能なインタラクティブメニューを追加。

### 変更 (Changed)
- `src/main.rs`: メインメニューに `[8] Select AI Search Algorithm (Ranked 1st to 8th)` を追加し、アクティブなアルゴリズム名をリアルタイム表示。`run_ai_mode` が選択されたアルゴリズムで動作するよう連携。
- `src/gpu.rs`: WGSL Compute Shader を 20次元非線形多項式ハイブリッド評価モデル対応へ拡張（`max_features = 32`）。
- `src/main.rs`: メインメニューおよびベンチマークランナーを ROCm / Vulkan 両対応へ更新。
- `src/benchmark.rs`: マイクロベンチマークおよびバックエンド別実戦比較機能を追加。

### 削除 (Removed)
- **人間が操作するプレイモード**: `run_play_mode`（エキスパートデータ収集用手動プレイ）および `run_free_play_mode`（フリープレイ練習モード）を削除。
- **模倣学習 (Imitation Learning)**: `src/imitation.rs`（行動クローニング、`dataset.json` の読み書き、SGD学習ループ）および関連メニュー項目を完全削除。

### テスト (Testing)
- `test_hip_evaluator_linear_and_nonlinear`: ROCm HIP カーネルの線形・非線形評価テストをパス。
- `test_20_features_extraction`: 20次元特徴量抽出テストをパス。
- `test_reachability_bfs_tspin_slot`: T-Spinスロット到達テストをパス。
- `test_gpu_evaluator_initialization`: GPU Compute Shader バッチ評価テストをパス。
- `test_gpu_beam_search_lookahead`: GPU Beam Search 先読みテストをパス。
- 全13単体テストおよび ROCm vs Vulkan ベンチマークを完走。
