# 🎮 High-Performance GPU-Accelerated Tetris AI System (tetris_ai)

[![Rust](https://img.shields.io/badge/Rust-2024%20Edition-orange.svg)](https://www.rust-lang.org/)
[![Vulkan](https://img.shields.io/badge/GPU%20Compute-Vulkan%20wgpu-blue.svg)](https://wgpu.rs/)
[![ROCm HIP](https://img.shields.io/badge/ROCm%20HIP-7.1%20gfx1200-red.svg)](https://rocm.docs.amd.com/)
[![Version](https://img.shields.io/badge/version-v0.1.4-green.svg)](https://github.com/sha256san/tetris-ai-udon)
[![Tests](https://img.shields.io/badge/Tests-32%2F32%20Pass-brightgreen.svg)]()

> **世界最強水準の火力維持・T-Spin連鎖・ドネイト判断・実戦操作高速化を備えた、ROCm HIP / Vulkan GPU並列加速テトリスAIエンジン**

---

## 🌟 主要機能とアーキテクチャ

### 1. ⚡ 実戦特化 操作高速化 & 0ディレイ着地 (`src/ai.rs`)
- **ハードドロップ優先原則 (`optimize_execution_path`)**: 上空から直線落下可能なオープン着地点に対して、無駄なソフトドロップを完全に排除し最短の `[Rotate, Shift, HardDrop]` 直通パスで即座に配置。
- **スピン・潜り込み完了後の即時ハードドロップ**: SRSウォールキックやソフトドロップで隙間に潜り込ませた後、接地判定待ち（0.5秒ロックディレイ）を発生させず、目標地点到達の瞬間に即座に `MoveAction::HardDrop` を発行して固定。

### 2. 🧠 20次元 非線形多項式ハイブリッド評価関数 (`src/ai.rs`, `src/gpu.rs`)
- **GPU並列高速バッチ評価**: Vulkan Compute Shader / AMD ROCm HIP (`gfx1200`) 上で毎秒1,500万手以上の候補手を並列評価。
- **特徴量**: T-Spin、T-Spin地形品質、穴数、穴深度、穴空間分散、着地品質、テトリス、Pure Single/Double/Triple抑制、REN、BTB、PC、総標高、最大標高、凹凸・**中央山型凸度抑制**、**両端同時空き防止**、不純オーバーハング、**将来適合度（HoldT/HoldI温存 & WasteT抑制）**。

### 3. 🎯 高度T-Spin & ドネイト（階段積み）理論 (`src/tetris.rs`)
- **中央山型（富士山型）凸度抑制**: 中央列（x=3..6）が高く盛り上がる地形にペナルティを付与し、外側への分散配置を促進。
- **両端同時空き防止**: 左端（x=0）と右端（x=9）が同時に深穴になる状態を致命的ペナルティとして排除し、Iミノ枯渇・窒息死を完全防止。
- **3〜8列目 単一穴スロット**: 穴幅1マスの内側Tスロットを最高評価し、T-Spin発火後もフラットな地形を維持。
- **壁端TSTの物理的内向き屋根制約**: 空中浮遊を排除し、盤面内側から壁に向かって屋根が伸びる内向き構造のみを有効認定。
- **しゑひ式 階段のドネイト (Kaidan Setups)**: 高低差1マスの段差を利用したS/Z/J/Lドネイトを検知し、2ライン保持則により下穴を安全に再開口。

### 4. 📊 T-Spin Mini 後の BTB 継続検証 & 直前1手強調ログ (`src/tspin_recorder.rs`)
- **BTB 追跡判定**: T-Spin Mini 発火後の5手を自動走査し、BTB を維持して本命火力（Tetris や TSD/TST）を発火できたかを判定（`Successful Mini -> BTB Maintained` または `Wasted Mini`）。
- **直前1手（Turn -1: 屋根仕込み手）の強調**: T-Spin発火直前の1手を視覚的な専用バナー `🔥🔥🔥 【T-SPIN SETUP MOVE: 1 TURN BEFORE TRIGGER (Turn X)】 🔥🔥🔥` で強調表示。
- **Mini無駄打ちペナルティ**: BTBや本命火力に繋がらない単発Miniを評価関数で抑制。

### 5. 🚀 4体同時 10,000回反復 並列進化学習バッチ (`run_parallel_training_10000.sh`)
- 4つのAIワーカープロセスを完全並行（バックグラウンド）で起動し、各ワーカーが10,000イテレーションずつ探索を実行。
- 各ラウンド終了時に全ワーカーのFitness・TSD/TST発火回数・VRAMチェックポイントを自動比較し、最高性能の重みを `model.json` に昇格させて次ラウンドへ自動継承する永続ループ。

### 6. 📚 1,180項目 構造化テトリス知識ベース (`tetris-ai-research/`, `src/knowledge.rs`)
- `addplan3.md` 準拠の全16カテゴリ・1,180件の詳細知識をJSON（`knowledge.json`, `terrain_patterns.json`）およびMarkdownリファレンス文書（`01_rules/` 〜 `12_sources/`）として完全体系化。
- Rustエンジンから `KnowledgeBase::load_from_default_path()` で高速ロード可能。

---

## 💻 動作環境・必要要件

- **OS**: Linux (Ubuntu 22.04 / 24.04 LTS 推奨) または Windows
- **Rust**: 1.80+ (2024 Edition)
- **GPU (任意・自動検出)**:
  - **ROCm HIP**: AMD Radeon RX 9000 / 7000 / 6000 シリーズ (ROCm 6.x / 7.x)
  - **Vulkan (wgpu)**: AMD / NVIDIA / Intel GPU (Vulkan 1.2+)

---

## 🛠️ クイックスタート & 実行コマンド

### 1. ビルド & テスト実行
```bash
# 全32件の単体テスト実行 (100% Pass)
cargo test

# リリースバイナリのビルド
cargo build --release
```

### 2. AI 自動プレイモード (Terminal UI / リアルタイム描画)
```bash
cargo run --release
# メニューから [1] AI Auto Play Mode を選択
```

### 3. 4体同時 並列進化学習バッチの起動 & 停止
```bash
# 【開始 (推奨: 500回/ラウンド)】4体のワーカーをバックグラウンド並行起動（500回ごとに最優秀モデルを model.json へ自動マージし、自動で次ラウンドへ永久継続）
./run_parallel_training.sh 500

# 【高速回転: 100回/ラウンド】
./run_parallel_training_100.sh  # または ./run_parallel_training.sh 100

# 【停止】実行中の全ワーカーとバッチを安全に停止し、最新の最良重みを model.json に保存
./stop_parallel_training.sh
```

> **自動継続（永続ループ）の仕組み**:
> - 各ラウンド（4体 × N回探索）が完了するたびに、4体の中で最も高いFitnessを達成したモデルを自動判定します。
> - 最優秀重みを `model.json` に上書き保存し、**自動的にRound 2、Round 3...へと最新の重みを引き継いで学習を繰り返します**。
> - 途中で止めたい場合は、端末で `Ctrl+C` を押すか、別端末から `./stop_parallel_training.sh` を実行するだけで安全に停止できます。

### 4. T-Spin 特化 1,000回 最適化チューニング (単体実行)
```bash
cargo run --release -- --tune-tspin 1000
```

### 5. ROCm HIP vs Vulkan 総合ベンチマーク実行
```bash
cargo run --release -- --benchmark
```

### 6. Web対戦サーバーの起動
```bash
cargo run --release
# メニューから [7] Start AI Battle Web Server を選択
# ブラウザで http://localhost:3000/battle/ を開く
```

---

## 📁 ディレクトリ構成

```text
tetris_ai/
├── Cargo.toml                         # プロジェクト設定 (v0.1.4)
├── model.json                         # 20特徴量最適化モデル重み
├── run_parallel_training_10000.sh     # 4体同時10,000回並列学習バッチ
├── scripts/
│   ├── merge_best_worker.py          # 並列ワーカー最優秀モデル選定・自動マージ
│   └── generate_tetris_research.py   # 1,180項目知識データセット生成器
├── src/
│   ├── ai.rs                          # 3D BFS到達探索・ハードドロップ最適化・評価関数
│   ├── tetris.rs                      # ゲームエンジン・T-Spin幾何学・階段ドネイト判定
│   ├── config.rs                      # 評価パラメータ・ペナルティ設定
│   ├── knowledge.rs                   # 1,180項目知識ベース管理モジュール
│   ├── tspin_recorder.rs              # T-Spin前後5手記録・BTB追跡・直前1手強調
│   ├── tuning.rs                      # マルチワーカー進化学習 & VRAM同期
│   ├── benchmark.rs                   # ROCm HIP vs Vulkan マイクロ/マクロベンチマーク
│   ├── gpu.rs                         # Vulkan WGSL Compute Shader
│   ├── hip.rs & hip_kernel.cpp        # AMD ROCm 7.1 HIP ネイティブカーネル
│   ├── server.rs                      # Web対戦サーバー (動的ポートバインド)
│   └── main.rs                        # CLI引数解析 & インタラクティブメニュー
└── tetris-ai-research/                # 1,180項目 構造化リサーチベース
    ├── 01_rules/rules.md              # ガイドライン・SRS・7-Bag仕様
    ├── 02_terrain/                    # 地形特徴量 & パターン集
    ├── 03_tspin/                      # T-Spin構造・壁端制約・ドネイト理論
    ├── 04_attack/                     # 火力・APM・APL理論
    ├── 05_openers/ 〜 09_ren/         # 開幕・中盤・ダウンスタック・PC・REN
    ├── 10_ai/                         # 評価関数・GPUビームサーチ・状態表現
    └── 11_dataset/                    # knowledge.json (1,180件) & terrain_patterns.json
```

---

## 📄 ライセンス

本プロジェクトは MIT License のもとで公開されています。
