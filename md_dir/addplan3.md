# 強いテトリスAI用 知識収集・分析タスク計画

## 目的

より強いテトリスAIを作るために必要な知識を体系的に収集する。

対象は単なる「テトリスをプレイできるAI」ではなく、以下を判断できるAIとする。

* 火力を継続して出せる
* T-Spinを狙える
* T-Spinドネイトを判断できる
* 地形から攻撃テンプレを発見できる
* ダウンスタックできる
* RENを適切に利用できる
* B2Bを維持できる
* パフェを狙える
* 相手の状況に応じて攻撃方針を変更できる
* ホールドとNEXTを活用できる
* 単純なテンプレ暗記ではなく地形理解によって判断できる

---

# Phase 0: 調査対象・ルールセットの確定

## Task 0-1: 対象ゲームルールの整理

以下のゲームルール差を調査・整理する。

* Tetris Guideline
* TETR.IO
* Puyo Puyo Tetris
* Jstris
* Tetris Effect
* ブラウザテトリス

### 収集する項目

* フィールドサイズ
* SRS仕様
* 7-bag
* Hold
* Next Queue
* B2B仕様
* REN仕様
* T-Spin判定
* Mini判定
* 攻撃力計算
* ガーベージ仕様
* ガーベージキャンセル
* ガーベージ穴の仕様
* ロックディレイ
* ARR
* DAS

---

# Phase 1: 既存テトリス知識サイトの完全分類

## Task 1-1: URLの収集

対象：

```text
https://shiwehi.com/tetris/
```

収集対象：

```text
/tetris/template/
/tetris/game/
/tetris/explanation/
```

除外：

```text
Google Ads
外部広告
トラッキングURL
SVG広告要素
```

---

## Task 1-2: 全記事をカテゴリ分類

各URLに以下のタグを付与する。

* opener
* midgame
* downstack
* t_spin
* t_spin_donate
* donate
* perfect_clear
* ren
* b2b
* stacking
* attack
* defense
* terrain
* spin
* knowledge
* tool

---

## Task 1-3: 記事から知識を構造化

各記事から以下を抽出する。

```text
テンプレ名
URL
カテゴリ
必要ミノ順
使用ミノ数
Hold必要性
T-Spin有無
T-Spin回数
ライン消去数
推定火力
B2B維持
REN発生
パフェ可能性
派生
失敗時のリカバリー
必要地形
終了地形
次の攻撃候補
```

---

# Phase 2: 大項目 約100個の知識体系作成

## A. 基本ゲームシステム

### Task 2-01 ～ 2-10

1. フィールド構造
2. ミノ形状
3. SRS
4. 7-bag
5. Hold
6. Next Queue
7. Lock Delay
8. DAS
9. ARR
10. 入力速度と判断速度

---

## B. 地形評価

### Task 2-11 ～ 2-20

11. Aggregate Height
12. Maximum Height
13. Height Variance
14. Holes
15. Hole Depth
16. Covered Holes
17. Bumpiness
18. Well Depth
19. Column Difference
20. Surface Roughness

---

## C. 穴・地形の危険度

### Task 2-21 ～ 2-30

21. 穴の数
22. 穴の深さ
23. 穴の横幅
24. 穴の分散
25. 穴の連結
26. 穴の上のブロック数
27. 穴へのアクセス性
28. 穴修復コスト
29. 将来穴発生リスク
30. 地形崩壊リスク

---

## D. T-Spin

### Task 2-31 ～ 2-40

31. T-Spin Single
32. T-Spin Double
33. T-Spin Triple
34. T-Spin Mini
35. T-Spinセットアップ
36. T-Spin地形検出
37. T-Spin継続
38. T-Spin連鎖
39. T-Spin失敗地形
40. T-Spin期待値

---

## E. T-Spin Donate

### Task 2-41 ～ 2-50

41. TSD Donate
42. TSS Donate
43. T-Spinへのドネイト
44. ドネイト後TSD
45. ドネイト後B2B
46. Donate Chain
47. 地形からDonate検出
48. Donate可能穴
49. Donate成功率
50. Donate失敗リスク

---

## F. 火力

### Task 2-51 ～ 2-60

51. Attack Per Minute
52. Attack Per Piece
53. Attack Per Line
54. T-Spin Efficiency
55. B2B Efficiency
56. REN Efficiency
57. Perfect Clear Efficiency
58. Spike Damage
59. Sustained Damage
60. Damage Efficiency

---

## G. B2B

### Task 2-61 ～ 2-65

61. B2B維持
62. B2B開始
63. B2B延長
64. B2B火力期待値
65. B2Bを切る判断

---

## H. REN

### Task 2-66 ～ 2-70

66. 4列REN
67. REN開始地形
68. REN継続地形
69. REN火力期待値
70. REN中断判断

---

## I. Perfect Clear

### Task 2-71 ～ 2-75

71. 開幕PC
72. 2巡PC
73. 3巡PC
74. PC確率
75. PC探索アルゴリズム

---

## J. 開幕戦略

### Task 2-76 ～ 2-85

76. DT砲
77. TSD
78. BT砲
79. PC Opener
80. Grace System
81. Mechanical TSD
82. Albatross
83. Stick Spin
84. 中開けREN
85. 開幕テンプレ比較

---

## K. 中盤戦略

### Task 2-86 ～ 2-95

86. Flat Stack
87. LST
88. STSD
89. DT
90. BT
91. Imperial Cross
92. Double Dagger
93. TPC
94. DPC
95. QPC

---

## L. ダウンスタック・防御

### Task 2-96 ～ 2-100

96. Garbage Downstack
97. Hole Access
98. Downstack Efficiency
99. Defensive Stacking
100. Counter Attack

---

# Phase 3: 詳細知識 約1000項目の収集

各大項目について約10個の詳細項目を収集する。

---

# 詳細項目の共通フォーマット

各知識を以下の構造で保存する。

```json
{
  "id": "terrain_001",
  "category": "terrain",
  "name": "Aggregate Height",
  "description": "各列の高さの合計",
  "importance": 0.8,
  "higher_is_better": false,
  "related_features": [
    "holes",
    "max_height"
  ],
  "ai_usage": [
    "evaluation_function",
    "reinforcement_learning"
  ],
  "source": []
}
```

---

# Phase 4: 地形特徴量の収集

## Task 4-1: 基本特徴量

最低限収集する。

```text
aggregate_height
max_height
min_height
height_variance
height_difference
bumpiness
holes
covered_holes
hole_depth
hole_width
well_depth
surface_roughness
```

---

## Task 4-2: AI向け高度特徴量

追加する。

```text
tspin_slot_count
tspin_ready_count
tspin_donate_count
donate_candidate_count

b2b_potential
b2b_continuation

ren_potential
ren_length_potential

pc_probability

attack_potential
spike_potential
sustained_attack_potential

downstack_accessibility

garbage_risk
topout_risk
```

---

# Phase 5: T-Spin検出システム

## Task 5-1: T-Spin地形検出

AIが盤面から以下を検出する。

```text
TSD Slot
TSS Slot
TST Slot
Mini Slot
```

---

## Task 5-2: T-Spin候補探索

各行動について以下を評価する。

```text
T-Spin可能
必要ミノ
Hold使用
必要回転
必要Kick
必要Next
火力
B2B
地形悪化
失敗リスク
```

---

# Phase 6: Donate検出システム

## Task 6-1: Donate候補生成

盤面から以下を探索する。

```text
TSD Donate
TSS Donate
Generic Donate
Line Clear Donate
B2B Donate
```

---

## Task 6-2: Donate評価

```text
attack_gain
terrain_cost
future_spin_potential
b2b_value
risk
recovery_cost
```

---

# Phase 7: 火力地形の研究

## Task 7-1: 高火力地形の定義

以下の地形を収集する。

* TSD連鎖可能地形
* TSS連鎖可能地形
* B2B維持地形
* REN開始地形
* REN継続地形
* Tetris Well
* PC遷移地形
* Donate可能地形

---

## Task 7-2: 火力期待値計算

各地形について、

```text
Expected Attack
Expected Lines
Attack Per Piece
Attack Per Second
B2B Probability
T-Spin Probability
```

を記録する。

---

# Phase 8: NEXT・Hold戦略

## Task 8-1: Next活用

AIが以下を判断する。

```text
1手読み
2手読み
3手読み
5手読み
7-bag予測
```

---

## Task 8-2: Hold戦略

評価する。

```text
Holdする価値
Hold温存
Tミノ保存
Iミノ保存
PC用Hold
T-Spin用Hold
緊急Hold
```

---

# Phase 9: 対戦AI戦略

## Task 9-1: 相手盤面分析

収集する。

```text
opponent_height
opponent_holes
opponent_b2b
opponent_ren
opponent_attack
opponent_danger
```

---

## Task 9-2: 攻撃判断

AIが判断する。

```text
immediate_attack
spike
sustained_attack
counter
defense
downstack
```

---

# Phase 10: 評価関数設計

最終的に以下の形式を目標とする。

```text
Score =
  TerrainScore
+ AttackPotential
+ TSpinPotential
+ DonatePotential
+ B2BPotential
+ RENPotential
+ PCPotential
+ DownstackScore
- HolePenalty
- HeightPenalty
- RiskPenalty
```

---

# Phase 11: 探索アルゴリズム

## Task 11-1: 行動探索

比較対象：

* Greedy
* Beam Search
* DFS
* BFS
* Expectimax
* Monte Carlo Tree Search

---

## Task 11-2: 探索深度

比較する。

```text
1手
2手
3手
5手
7手
10手
```

評価項目：

* 勝率
* PPS
* APM
* APL
* T-Spin回数
* T-Spin成功率
* B2B
* REN
* Topout率

---

# Phase 12: AI評価指標

最低20項目以上を測定する。

```text
1. Win Rate
2. Topout Rate
3. PPS
4. APM
5. APL
6. APP
7. T-Spin Count
8. T-Spin Success Rate
9. T-Spin Donate Count
10. Donate Success Rate
11. Tetris Count
12. B2B Length
13. B2B Count
14. REN Count
15. REN Max
16. Perfect Clear Count
17. Hole Count
18. Average Height
19. Maximum Height
20. Downstack Efficiency
21. Attack Efficiency
22. Garbage Efficiency
23. Decision Time
24. Search Nodes
25. Search Depth
```

---

# Phase 13: データセット化

## データ形式

```json
{
  "board": [],
  "current_piece": "T",
  "hold_piece": "I",
  "next_queue": [],
  "action": {
    "rotation": 1,
    "x": 4
  },
  "evaluation": {
    "score": 0.0,
    "attack": 0,
    "tspin": false,
    "donate": false
  }
}
```

---

# Phase 14: 約1000項目の完成条件

知識項目を以下の分類で最低限収集する。

| 分野            | 目標項目数 |
| ------------- | ----: |
| 基本ルール         |    50 |
| 地形評価          |   150 |
| 穴・危険地形        |    80 |
| T-Spin        |   120 |
| Donate        |   100 |
| 火力            |   100 |
| B2B           |    50 |
| REN           |    50 |
| Perfect Clear |    80 |
| 開幕テンプレ        |    80 |
| 中盤テンプレ        |    70 |
| ダウンスタック       |    80 |
| NEXT/Hold     |    40 |
| 対戦戦略          |    60 |
| 探索アルゴリズム      |    40 |
| AI評価          |    30 |

**合計：約1,160項目**

---

# 最終成果物

```text
tetris-ai-research/
│
├── README.md
│
├── 01_rules/
│   └── rules.md
│
├── 02_terrain/
│   ├── terrain_features.md
│   └── terrain_patterns.md
│
├── 03_tspin/
│   ├── tspin.md
│   └── tspin_donate.md
│
├── 04_attack/
│   ├── attack.md
│   └── firepower_terrain.md
│
├── 05_openers/
│   └── openers.md
│
├── 06_midgame/
│   └── midgame.md
│
├── 07_downstack/
│   └── downstack.md
│
├── 08_pc/
│   └── perfect_clear.md
│
├── 09_ren/
│   └── ren.md
│
├── 10_ai/
│   ├── evaluation.md
│   ├── search.md
│   ├── state_representation.md
│   └── training.md
│
├── 11_dataset/
│   ├── knowledge.json
│   └── terrain_patterns.json
│
├── 12_sources/
│   └── sources.md
│
└── TODO.md
```

---

# 優先順位

## Priority 1

最初に実装・調査する。

```text
地形評価
穴
T-Spin
T-Spin Donate
火力地形
B2B
Next/Hold
探索アルゴリズム
```

---

## Priority 2

次に追加する。

```text
REN
Perfect Clear
開幕テンプレ
中盤テンプレ
Downstack
```

---

## Priority 3

高度な対戦AI化。

```text
相手盤面認識
Garbage予測
Counter
Spike Timing
Defense Timing
Adaptive Strategy
```

---

# 完成基準

最終的なAIは、

1. 現在の盤面を解析する
2. 地形特徴量を生成する
3. T-Spin候補を検出する
4. Donate候補を検出する
5. 火力期待値を計算する
6. B2B継続可能性を評価する
7. NEXTとHoldを考慮する
8. 複数手先を探索する
9. 相手盤面を評価する
10. 最適な攻撃・防御戦略を選択する

というパイプラインを持つことを目標とする。

```text
Game State
    ↓
Board Analysis
    ↓
Terrain Feature Extraction
    ↓
Pattern Detection
    ├── T-Spin
    ├── Donate
    ├── REN
    ├── PC
    └── Downstack
    ↓
Candidate Generation
    ↓
Multi-Step Search
    ↓
Evaluation Function
    ↓
Opponent Analysis
    ↓
Action Selection
```
