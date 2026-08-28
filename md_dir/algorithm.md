# Tetris AI 探索アルゴリズム探索計画書

## 1. 目的

テトリスAIにおいて、盤面から最適な行動を選択するための「探索アルゴリズム」を複数実装し、

- 生存時間
- 消去ライン数
- スコア
- 1秒あたりの探索ノード数
- 1手あたりの計算時間
- メモリ使用量
- 探索の安定性
- 盤面の穴・凹凸・積み上がりに対する耐性

を比較する。

最終的には、**自分のテトリス実装・使用CPU/GPU・評価関数に最も適した探索アルゴリズムを決定する**。

---

# 2. まず比較対象にする探索アルゴリズム

最低でも以下の5方式を調査・実装する。

| # | 方法 | 概要 | 実装難易度 | 探索能力 | 計算量 |
|---|---|---|---|---|---|
| 1 | Beam Search | 上位候補だけを残しながら複数手先を探索 | 低〜中 | 高 | 中 |
| 2 | Monte Carlo Tree Search (MCTS) | ランダムシミュレーションを大量に行い有望な手を探索 | 中〜高 | 高 | 高 |
| 3 | Minimax / Expectimax | 次ピースなどの不確実性を考慮して探索 | 中 | 高 | 高 |
| 4 | Genetic Algorithm | 探索パラメータや行動列を進化させる | 中 | 中〜高 | 高 |
| 5 | A* / Best-First Search | 評価関数を使って有望な盤面から優先探索 | 中 | 中〜高 | 中〜高 |
| 6 | Breadth-First Search (BFS) | 同じ深さのノードをすべて探索 | 低 | 低〜中 | 非常に高 |
| 7 | Depth-Limited DFS | 指定深さまで深く探索 | 低 | 中 | 低〜中 |

最初の比較では **Beam Search / MCTS / Expectimax / Genetic Algorithm / A*** の5方式を主要候補とする。

---

# 3. 方法1: Beam Search

## 3.1 概要

各手について未来の盤面を生成し、その中から評価値の高い上位 `K` 個だけを残して次の深さを探索する。

例:

```text
現在盤面
   │
   ├─ 手A
   ├─ 手B
   ├─ 手C
   ├─ 手D
   └─ 手E
        ↓
   評価関数で順位付け
        ↓
   上位K個だけ残す
        ↓
   次のミノ探索
```

## 3.2 調査するパラメータ

- Beam Width = 10
- Beam Width = 50
- Beam Width = 100
- Beam Width = 500

さらに探索深さを:

- 1手先
- 2手先
- 3手先
- 4手先
- 5手先

で比較する。

## 3.3 メリット

- 実装しやすい
- 将来の盤面を複数比較できる
- 探索幅を制限できる
- Tetrisとの相性が良い可能性が高い

## 3.4 デメリット

- Beam Widthが小さいと有望な手を途中で捨てる
- Beam Widthが大きいと計算量が急増する

## 3.5 最初に実装する探索方式

**最初のベースラインとしてBeam Searchを実装する。**

---

# 4. 方法2: Monte Carlo Tree Search (MCTS)

## 4.1 概要

盤面から複数の手を試し、ランダムまたは半ランダムにゲームを進める。

大量のシミュレーションを実行し、

```text
この手を選ぶ
 ↓
ランダムにプレイ
 ↓
どれくらい生き残ったか
 ↓
結果を蓄積
```

という方法で最も期待値の高い行動を選択する。

## 4.2 基本構成

MCTSでは以下の4段階を使用する。

1. Selection
2. Expansion
3. Simulation
4. Backpropagation

## 4.3 調査するパラメータ

- 1手あたり100 simulation
- 1手あたり500 simulation
- 1手あたり1,000 simulation
- 1手あたり5,000 simulation
- UCT係数

## 4.4 メリット

- 評価関数への依存を減らせる
- 深い未来を考慮できる
- 未知の戦略を発見できる可能性がある

## 4.5 デメリット

- 計算量が大きい
- テトリスではランダムシミュレーションの質が重要
- 良いRollout Policyが必要

## 4.6 GPU活用

MCTSは大量の独立シミュレーションを実行できるため、

```text
CPU
 └─ Tree Management

GPU
 ├─ Simulation 1
 ├─ Simulation 2
 ├─ Simulation 3
 ├─ ...
 └─ Simulation N
```

という構成も検討する。

---

# 5. 方法3: Expectimax / Minimax

## 5.1 概要

テトリスでは次に出るミノが完全には制御できない。

そのため、

```text
AIの手
 ↓
次のミノ候補
 ↓
AIの手
 ↓
次のミノ候補
```

という「意思決定 + 不確実性」の木を作る。

通常のMinimaxよりも、テトリスでは**Expectimax**が適している可能性がある。

## 5.2 Expectimaxのイメージ

```text
                現在盤面
                    │
             AIが行動を選択
          ┌────────┼────────┐
          A         B        C
          │         │        │
       次ミノ      次ミノ    次ミノ
      ┌──┼──┐    ┌──┼──┐
      I  O  T     I  O  T
```

それぞれの結果を確率付きで評価する。

## 5.3 調査条件

探索深さ:

- 1
- 2
- 3
- 4
- 5

次ミノ情報:

- 現在のミノのみ
- Next 1
- Next 2
- Next 3
- Next 5

## 5.4 メリット

- 「次に何が来るか」というテトリス特有の不確実性を扱える
- 先読みが強い
- 理論的に比較しやすい

## 5.5 デメリット

- 探索木が非常に大きくなる
- 高速な盤面評価が必要

## 5.6 最適化

以下を導入する。

- Transposition Table
- State Hashing
- Alpha-Beta Pruning（Minimax系に適用）
- Beam Pruning
- 並列探索

---

# 6. 方法4: Genetic Algorithm

## 6.1 概要

探索アルゴリズムそのものではなく、

**「どの行動・評価パラメータを使えば強いAIになるか」**

を進化計算で探索する。

例えば評価関数:

```text
score =
    a * lines_cleared
  - b * aggregate_height
  - c * holes
  - d * bumpiness
  - e * well_depth
```

に対して、

```text
[a, b, c, d, e]
```

を遺伝子として扱う。

## 6.2 進化の流れ

```text
ランダムな評価関数を100個生成
          ↓
       テトリスをプレイ
          ↓
       成績を測定
          ↓
        上位を選択
          ↓
        交叉・突然変異
          ↓
      新しい100個を生成
          ↓
        再評価
```

## 6.3 調査対象

評価関数の重みだけではなく、

- 探索深さ
- Beam Width
- MCTS simulation数
- 行動候補数
- 盤面特徴量
- Next Pieceの利用数

も遺伝子化する。

## 6.4 メリット

- 人間が最適な重みを決めなくてもよい
- 強い評価関数を自動探索できる
- 他の探索法と組み合わせられる

## 6.5 デメリット

- 学習回数が非常に多い
- フィットネス評価に時間がかかる
- 局所最適に陥る可能性がある

---

# 7. 方法5: A* / Best-First Search

## 7.1 概要

盤面の状態に対して、

```text
f(n) = g(n) + h(n)
```

を計算し、有望なノードから探索する。

- `g(n)` = ここまでのコスト
- `h(n)` = 将来コストの推定

テトリスでは例えば、

```text
h(n) =
    holes
  + aggregate_height
  + bumpiness
  + max_height
```

などを利用する。

## 7.2 Best-First Search

A*のコスト計算を簡略化し、

```text
評価値が最も高い盤面
```

を優先して探索する。

## 7.3 メリット

- 実装しやすい
- 優先探索と相性が良い
- 評価関数を明示的に制御できる

## 7.4 デメリット

- 良いヒューリスティックが必要
- 探索空間が大きくなるとメモリ消費が増える

---

# 8. 方法6: BFS

## 8.1 概要

現在の盤面から可能な行動をすべて展開し、

```text
深さ0
 ↓
深さ1
 ↓
深さ2
 ↓
深さ3
```

と全探索する。

## 8.2 使用目的

BFSは最終的な高速AIとしてではなく、

**他の探索アルゴリズムが正しいかを確認するGround Truth / 比較基準**

として使用する。

## 8.3 メリット

- 実装が簡単
- 探索漏れが少ない
- 他アルゴリズムの検証に使いやすい

## 8.4 デメリット

- 探索ノード数が爆発する
- 深い探索に向かない

---

# 9. 方法7: Depth-Limited DFS

## 9.1 概要

一定の深さまでDFSを実行し、末端状態を評価する。

```text
現在
 ├─ A
 │  ├─ A1
 │  └─ A2
 ├─ B
 │  ├─ B1
 │  └─ B2
 └─ C
```

## 9.2 使用目的

- ベースライン実装
- Beam Searchとの比較
- 探索深さによる性能変化の測定

---

# 10. 盤面評価関数

探索アルゴリズムを比較するときは、探索方式だけを変え、評価関数をなるべく統一する。

基本評価関数:

```text
evaluation =
      w1 * lines_cleared
    - w2 * aggregate_height
    - w3 * holes
    - w4 * bumpiness
    - w5 * max_height
    - w6 * blockades
    - w7 * well_depth
```

## 10.1 最低限必要な特徴量

- 消去ライン数
- 穴の数
- 総高さ
- 最大高さ
- 凹凸
- ホールの深さ
- Blockades
- Well Depth

## 10.2 将来追加する特徴量

- T-spin potential
- Combo potential
- Back-to-back potential
- Perfect Clear potential
- Tetris potential
- 4列構造
- Overhang
- Surface entropy

---

# 11. 共通ゲーム環境を作る

探索アルゴリズムの比較で最も重要なのは、

**同じ条件で比較すること**

である。

すべてのアルゴリズムで以下を共通化する。

```text
Board
Piece Generator
Collision Detection
Rotation
Line Clear
Hold
Next Queue
Scoring
Random Seed
```

探索アルゴリズムだけ差し替えられる設計にする。

例:

```text
TetrisEngine
 ├── Board
 ├── PieceGenerator
 ├── GameRules
 └── SearchPolicy
       ├── BeamSearch
       ├── MCTS
       ├── Expectimax
       ├── GeneticSearch
       ├── AStar
       ├── BFS
       └── DFS
```

---

# 12. 探索アルゴリズム比較方法

## 12.1 固定シード方式

同じミノ列を全AIに与える。

例:

```text
Seed = 1
Seed = 2
Seed = 3
...
Seed = 1000
```

これにより、単純な運による差を減らす。

---

# 13. 評価指標

| 指標 | 内容 |
|---|---|
| Lines | 消去ライン数 |
| Score | ゲームスコア |
| Survival Pieces | 何ミノ生存したか |
| Max Height | 最大積み上がり高さ |
| Holes | 穴の数 |
| Tetris Count | 4ライン消去回数 |
| PC Count | Perfect Clear回数 |
| Combo | 最大Combo |
| PPS | Pieces Per Second |
| Search Nodes | 探索ノード数 |
| Search Time | 1手あたり探索時間 |
| Memory | 使用メモリ |
| Win Rate | 指定条件での成功率 |

---

# 14. 重要な比較実験

## Experiment A: 探索深度比較

全アルゴリズムで:

```text
Depth = 1
Depth = 2
Depth = 3
Depth = 4
Depth = 5
```

を比較する。

目的:

**深く読むことでどれだけ強くなるかを測定する。**

---

## Experiment B: 計算時間固定

各AIの探索時間を固定する。

例:

```text
1 ms
5 ms
10 ms
50 ms
100 ms
500 ms
1000 ms
```

その時間内で最大限探索させる。

目的:

**同じ計算資源で最も強いアルゴリズムを探す。**

これは非常に重要な比較方法。

---

## Experiment C: ノード数固定

各AIで、

```text
1,000 nodes
10,000 nodes
100,000 nodes
1,000,000 nodes
```

のように探索ノード数を固定する。

目的:

**探索効率を比較する。**

---

## Experiment D: メモリ制限

例えば:

```text
256 MB
512 MB
1 GB
2 GB
```

などで制限して比較する。

---

## Experiment E: ミノ列固定

同じReplayを使用する。

```text
Replay #001
Replay #002
...
Replay #1000
```

同じ盤面・ミノ列に対する判断を比較する。

---

# 15. 探索アルゴリズムを自動的に見つける方法

最終的には「人間が1つ選ぶ」のではなく、

**自動ベンチマークシステム**

を作る。

```text
                ┌────────────┐
                │ Test Cases │
                └─────┬──────┘
                      ↓
        ┌────────────────────────┐
        │ Search Algorithm Runner │
        └────────────┬───────────┘
                     ↓
     ┌───────────────┼───────────────┐
     ↓               ↓               ↓
 Beam Search       MCTS          Expectimax
     ↓               ↓               ↓
 Genetic          A*              BFS
     └───────────────┼───────────────┘
                     ↓
              Benchmark System
                     ↓
              Statistical Analysis
                     ↓
                 Ranking
```

---

# 16. 自動探索するパラメータ

アルゴリズムだけではなく、以下も自動探索する。

## Beam Search

```text
depth
beam_width
evaluation_weights
```

## MCTS

```text
simulation_count
exploration_constant
rollout_policy
```

## Expectimax

```text
depth
next_piece_count
pruning
evaluation_weights
```

## A*

```text
heuristic_weights
search_depth
node_limit
```

## Genetic Algorithm

```text
population_size
mutation_rate
generation_count
selection_method
```

---

# 17. 探索アルゴリズム同士を組み合わせる

単純な5方式比較だけではなく、Hybrid方式を作る。

## Hybrid 1

```text
Beam Search
     ↓
MCTS
```

Beam Searchで候補を絞り、その候補だけMCTSで深く探索する。

## Hybrid 2

```text
Expectimax
     ↓
Beam Search
```

Expectimaxの探索木をBeam Searchで枝刈りする。

## Hybrid 3

```text
Genetic Algorithm
     ↓
Evaluation Function
     ↓
Beam Search
```

GAで評価関数を最適化し、Beam Searchがその評価関数を利用する。

## Hybrid 4

```text
A*
 ↓
MCTS
```

A*で有望な盤面を優先し、MCTSで未来を評価する。

---

# 18. GPUを利用した探索

GPUを使える環境では、大量の盤面評価をGPUへ移す。

特に並列化しやすい処理:

```text
Board Evaluation
Collision Check
Line Clear
Feature Extraction
Monte Carlo Simulation
Genetic Population Evaluation
```

GPUでは、

```text
Board 1
Board 2
Board 3
...
Board N
```

を並列評価する。

探索木そのものをGPUへ移すのではなく、

**大量の盤面評価をGPU化する設計**

から始める。

---

# 19. 最終的な選定基準

単純に「最も高スコアのAI」を選ばない。

以下を総合評価する。

```text
Final Score =
    0.40 * Playing Strength
  + 0.20 * Stability
  + 0.15 * Search Efficiency
  + 0.10 * Speed
  + 0.10 * Memory Efficiency
  + 0.05 * Implementation Complexity
```

用途に応じて重みは変更する。

---

# 20. 推奨する開発順序

## Phase 1

### ベース環境

- Tetris Engine完成
- Replay機能
- 固定Random Seed
- 盤面Hash
- 評価関数
- Benchmark Framework

---

## Phase 2

### ベースライン

実装:

1. DFS
2. BFS
3. Greedy Search

まず簡単な方式で性能基準を作る。

---

## Phase 3

### 本命アルゴリズム

実装:

1. Beam Search
2. A*
3. Expectimax
4. MCTS
5. Genetic Algorithm

---

## Phase 4

### 自動ベンチマーク

100〜1000種類の固定Replayでテスト。

各方式について:

```text
平均Lines
中央値Lines
最高Lines
平均Score
平均PPS
平均SearchTime
平均NodeCount
```

を収集する。

---

## Phase 5

### パラメータ探索

各方式について自動探索する。

例:

```text
Beam Width:
10 / 50 / 100 / 500

Depth:
1 / 2 / 3 / 4 / 5

Weights:
ランダム生成
```

さらに必要であればOptunaなどのハイパーパラメータ最適化を利用する。

---

## Phase 6

### Hybrid探索

上位2〜3方式を組み合わせる。

例:

```text
Beam + Expectimax
Beam + MCTS
GA + Beam
GA + Expectimax
```

---

# 21. 実験結果の保存形式

JSONまたはCSVで保存する。

例:

```json
{
  "algorithm": "BeamSearch",
  "depth": 3,
  "beam_width": 100,
  "seed": 123,
  "lines": 1842,
  "score": 923840,
  "pieces": 2317,
  "pps": 850.2,
  "search_time_ms": 1.37,
  "nodes": 42800
}
```

これを大量に保存し、Pythonなどで統計解析する。

---

# 22. 最終ランキング

最終的に以下のランキングを作る。

```text
Rank 1  : Algorithm X
Rank 2  : Algorithm Y
Rank 3  : Algorithm Z
```

ただし、

```text
最強
最速
最もメモリ効率が良い
最も安定
最も実装しやすい
```

を別々に評価する。

---

# 23. 最終目標

最終的な目標は、

> 「テトリスAIに最適な探索アルゴリズムを人間の勘で決める」のではなく、
> 「同一条件のベンチマークによって探索方式・探索深度・パラメータ・評価関数を自動比較し、最適な組み合わせを発見する」

ことである。

最終構成は以下を目標とする。

```text
              Tetris AI Benchmark
                      │
        ┌─────────────┼─────────────┐
        ↓             ↓             ↓
   Algorithm       Parameters     Evaluation
        │             │             │
        └─────────────┼─────────────┘
                      ↓
               Auto Benchmark
                      ↓
              Statistical Test
                      ↓
                 Ranking
                      ↓
             Best Configuration
```

---

# 24. 優先順位

実装優先度は以下とする。

| 優先度 | 手法 | 理由 |
|---|---|---|
| 1 | Beam Search | 実装と調整が容易で強力 |
| 2 | Expectimax | テトリスの確率的なミノ生成と相性が良い |
| 3 | A* / Best-First | 探索効率の比較に適する |
| 4 | MCTS | 深い探索・別系統の評価が可能 |
| 5 | Genetic Algorithm | 評価関数・探索パラメータの自動最適化 |
| 6 | DFS | ベースライン |
| 7 | BFS | 検証・比較用 |

---

# 25. 最初に作るべき最小構成

まず以下だけを完成させる。

```text
Tetris Engine
     ↓
Board State
     ↓
Feature Extraction
     ↓
Evaluation Function
     ↓
Beam Search
     ↓
Replay Benchmark
     ↓
CSV/JSON Logging
```

これが完成した時点で、

```text
Beam Width
Depth
Evaluation Weight
```

を変えながら性能を測定できる。

その後、

```text
Expectimax
MCTS
A*
Genetic Algorithm
```

を追加し、同一条件で比較する。

---

# 26. 成功条件

この計画は、以下を満たした時点で成功とする。

- 5種類以上の探索アルゴリズムを実装
- 同一Random Seedで比較可能
- 100〜1000以上のReplayで評価可能
- 探索時間を計測可能
- 探索ノード数を計測可能
- スコア・ライン数・生存ミノ数を記録可能
- パラメータを自動変更して再実験可能
- 最終的に最適な探索方式をランキングできる

---

# 結論

テトリスAIの探索アルゴリズムを見つける方法として、最低でも以下を比較する。

1. **Beam Search**
2. **MCTS**
3. **Expectimax / Minimax**
4. **Genetic Algorithm**
5. **A* / Best-First Search**
6. **BFS**
7. **Depth-Limited DFS**

最初から複雑なAIを作るのではなく、

```text
DFS/BFS
    ↓
Beam Search
    ↓
Expectimax / A*
    ↓
MCTS
    ↓
Genetic Algorithm
    ↓
Hybrid Search
    ↓
自動ベンチマーク
```

の順番で実装する。

特に重要なのは、**アルゴリズムの比較条件を統一すること**と、**探索時間・探索ノード数を固定した比較を行うこと**である。

最終的には「一番スコアが高かったアルゴリズム」ではなく、

**同じ計算資源で最も高いプレイ性能を出せる探索アルゴリズム + 評価関数 + パラメータの組み合わせ**

を採用する。
