# 変更履歴 (CHANGELOG.md)

すべての変更内容は本ドキュメントに記録されます。

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
