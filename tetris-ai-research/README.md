# Strong Tetris AI Knowledge & Strategy Research Base

本リサーチベースは、[addplan3.md](file:///home/sha256san/tetris_ai/md_dir/addplan3.md) に基づき、強いテトリスAIを構築するために必要な **合計 1180 項目** の体系的知識・地形パターン・評価指標・探索アルゴリズムを完全構造化したものです。

---

## 📊 知識分類・項目数サマリー

| No | カテゴリ | 項目数 | 説明・参照元 |
|---|---|---|---|
| 01 | **基本ルール (Rules)** | 50 項目 | ガイドライン・SRS・7-bag・各ゲーム仕様 |
| 02 | **地形評価 (Terrain)** | 150 項目 | 平坦度・中央凸度抑制・単一列穴・標高分布 |
| 03 | **穴・危険度 (Hazard)** | 80 項目 | 埋まった穴・両端同時空き・修復コスト |
| 04 | **T-Spin (T-Spin Mechanics)** | 120 項目 | TSD・TST・TSS・壁端内向きTST物理制約 |
| 05 | **ドネイト (T-Spin Donate)** | 100 項目 | 階段ドネイト・欄干・2ライン保持則 |
| 06 | **火力・効率 (Firepower)** | 100 項目 | APM・APL・Spike火力・持続火力 |
| 07 | **B2B (Back-to-Back)** | 50 項目 | B2B維持・B2B連鎖・切断判断 |
| 08 | **REN (Combo)** | 50 項目 | 4列REN・センターREN・REN中断判断 |
| 09 | **Perfect Clear (PC)** | 80 項目 | 開幕PC・2巡目PC・DPC/QPC確率 |
| 10 | **開幕戦術 (Openers)** | 80 項目 | DT砲・BT砲・TKI・MKO・メカニカルTSD |
| 11 | **中盤戦術 (Midgame)** | 70 項目 | LST積み・平積み・6-3積み・カウンター |
| 12 | **ダウンスタック (Downstack)** | 80 項目 | チーズ回収・穴開口・防御的ライン消去 |
| 13 | **NEXT・Hold (Queue)** | 40 項目 | HoldT温存・HoldI温存・7-bag周期予測 |
| 14 | **対戦戦略 (Battle AI)** | 60 項目 | 相手盤面分析・相殺キャンセル・Spike攻撃 |
| 15 | **探索アルゴリズム (Search)** | 40 項目 | GPU並列ビームサーチ・3D BFS・MCTS |
| 16 | **AI評価指標 (Metrics)** | 30 項目 | 25指標ベンチマーク・Fitness関数 |

**総計: 1180 知識項目 (目標 1,160 項目 達成)**

---

## 📁 ディレクトリ構成

- `01_rules/rules.md`: ガイドライン・SRSキック表・対戦仕様
- `02_terrain/terrain_features.md`, `terrain_patterns.md`: 地形特徴量とパターン集
- `03_tspin/tspin.md`, `tspin_donate.md`: T-Spin構造・壁端内向き制約・ドネイト
- `04_attack/attack.md`, `firepower_terrain.md`: 火力計算・APM理論
- `05_openers/openers.md`: 開幕テンプレ集
- `06_midgame/midgame.md`: 中盤積み・LST・平積み
- `07_downstack/downstack.md`: ダウンスタック理論
- `08_pc/perfect_clear.md`: パーフェクトクリア解法
- `09_ren/ren.md`: 4列REN・コンボ戦略
- `10_ai/`: 評価関数設計・探索・状態表現・強化学習
- `11_dataset/knowledge.json`, `terrain_patterns.json`: 機械可読データセット
- `12_sources/sources.md`: 引用文献・参考サイト一覧
