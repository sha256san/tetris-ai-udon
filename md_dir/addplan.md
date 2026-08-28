# Tetris AI 評価指標・非線形評価関数 計画書

## 1. 目的

テトリスAIの評価を「消去ライン数」だけで判断せず、攻撃性能・盤面安定性・T-spin地形・配置品質・将来性を総合評価する。

## 2. 主要20評価項目

| # | 指標 | 方向 | 主な意味 |
|---:|---|---|---|
| 1 | T-spin回数 | ↑ | T-spin Single/Double/Triple、Miniを区別 |
| 2 | T-spin地形品質 | ↑ | T-slot、角支持、回転可能性、深さ |
| 3 | 穴の数 | ↓ | 下にブロックがある空セル |
| 4 | 穴のばらけ具合 | ↓ | 穴の列・空間分散 |
| 5 | 置いた場所の評価 | ↑ | 着地点と地形・将来性の適合 |
| 6 | Tetris回数 | ↑ | 4ライン消去 |
| 7 | Pure Single回数 | ↓ | REN/T-spinを除くSingle |
| 8 | Pure Double回数 | ↓ | REN/T-spinを除くDouble |
| 9 | Pure Triple回数 | ↓ | REN/T-spinを除くTriple |
| 10 | REN | ↑ | 連続ライン消去 |
| 11 | BTB | ↑ | Tetris/T-spin系の継続 |
| 12 | 最大REN / Combo | ↑ | 瞬間的な連続性能 |
| 13 | 平均Combo | ↑ | 長期的な連続性能 |
| 14 | Perfect Clear | ↑ | PC回数・PC到達可能性 |
| 15 | 盤面総高さ | ↓ | Aggregate Height |
| 16 | 最大列高さ | ↓ | Max Height |
| 17 | 地形凹凸 | ↓ | Bumpiness |
| 18 | 井戸品質 | ↑/↓ | Tetris用井戸の理想状態 |
| 19 | オーバーハング | ↓ | 浮き・埋めにくい構造 |
| 20 | 将来手との適合度 | ↑ | Next/Holdを含むFuture Fit |

### 追加ログ候補

Hole Depth、Buried Hole、Blockade、Row/Column Transition、Surface Entropy、Reachability、Hold利用効率、Next Piece適合度、Tetris効率、攻撃ライン数、1秒あたり攻撃ライン数、T-spin Mini、T-spin Single/Double/Triple。

---

# 3. 各特徴量の定義

## T-spin

```text
TSpin =
    a1*TSpinSingle
  + a2*TSpinDouble
  + a3*TSpinTriple
  + a4*TSpinMini
```

## T-spin地形

```text
TSpinTerrain =
    w1*TSlot
  + w2*CornerSupport
  + w3*RotationAccessibility
  + w4*TSpinDepth
  - w5*UnfillableTSlot
```

## 穴

```text
Hole       = 総穴数
HoleDepth  = 平均/最大穴深度
BuriedHole = 深く埋まった穴
```

## 穴のばらけ具合

列ごとの穴数を `h_i` とすると、

```text
HoleVariance = (1/10) * Σ(h_i - mean(h))^2
```

さらに穴同士の平均マンハッタン距離を使う。

```text
HoleSpread = average(ManhattanDistance(hole_i, hole_j))
```

## Placement Quality

```text
Placement =
    p1*LinePotential
  + p2*TSpinPotential
  + p3*WellPotential
  + p4*FutureMobility
  - p5*NewHoles
  - p6*HeightIncrease
  - p7*BumpinessIncrease
```

## Tetris効率

```text
TetrisEfficiency = TetrisCount / PiecesPlaced
```

## REN

```text
RENScore =
    r1*MaxREN
  + r2*MeanREN
  + r3*RENContinuation
```

## BTB

```text
BTBScore =
    b1*BTBCount
  + b2*MaxBTB
  + b3*BTBContinuation
```

## Perfect Clear

```text
PCScore =
    c1*PCCount
  + c2*PCProbability
  + c3*NearPCCount
```

## 高さ

```text
AggregateHeight = Σ height_i
MaxHeight       = max(height_i)
```

## 凹凸

```text
Bumpiness = Σ |h_i - h_(i+1)|
```

## 井戸

「深ければ深いほど良い」ではなく理想深度を設定する。

```text
WellQuality =
    exp(-((WellDepth - D_opt)^2)/(2*sigma^2))
```

## オーバーハング

```text
OverhangPenalty =
    O
  + d*O^2
  + e*UnfillableOverhang^3
```

## Future Fit

```text
FutureFit =
    average(
        BestPlacement(Next1),
        BestPlacement(Next2),
        BestPlacement(Next3),
        BestPlacement(Hold)
    )
```

---

# 4. 20変数

特徴量を正規化する。

```text
x1  = TSpin
x2  = TSpinTerrain
x3  = HolePenalty
x4  = HoleSpreadPenalty
x5  = PlacementQuality
x6  = Tetris
x7  = PureSinglePenalty
x8  = PureDoublePenalty
x9  = PureTriplePenalty
x10 = REN
x11 = BTB
x12 = MaxCombo
x13 = MeanCombo
x14 = PC
x15 = HeightPenalty
x16 = MaxHeightPenalty
x17 = BumpinessPenalty
x18 = WellQuality
x19 = OverhangPenalty
x20 = FutureFit
```

`[0,1]` または `[-1,1]` に正規化してスケール差を抑える。

---

# 5. 一次関数だけにしない理由

単純な、

```text
F(X) = Σ wi*xi
```

だけでは指標間の相乗効果を表現できない。

例:

```text
T-spin地形
× Next T
× BTB維持
```

や、

```text
Tetris用井戸
× Iミノ
× BTB
```

は単純加算以上の価値を持つ可能性がある。

---

# 6. 推奨する二次多変量評価関数

```text
F2(X)
=
Σ wi*xi
+
Σ(i<j) aij*xi*xj
-
Σ pi*xi^2
```

- `wi`: 基本重み
- `aij`: 2特徴量の相互作用
- `pi`: 過剰評価を抑える係数

---

# 7. 三次項

重要な戦略について三次項を追加する。

```text
F3(X)
=
F2(X)
+
Σ bijk*xi*xj*xk
```

優先候補:

```text
TSpin * TSpinTerrain * FutureFit
Tetris * WellQuality * BTB
REN * Combo * FutureFit
Hole * HoleSpread * HoleDepth
Placement * FutureFit * NextFit
```

---

# 8. 四次項

全組み合わせには使わず、意味のある戦略だけに限定する。

```text
F4(X)
=
F3(X)
+
Σ cijkl*xi*xj*xk*xl
```

例:

```text
Tetris
× WellQuality
× BTB
× FutureFit
```

---

# 9. 非線形ペナルティ

## 高さ

```text
D_height = exp(k*(MaxHeight - H_safe))
```

または、

```text
D_height = sigmoid(k*(MaxHeight - H_safe))
```

## 穴

```text
D_hole =
    H
  + a*H^2
  + b*H^3
```

## 凹凸

```text
D_bump = B + c*B^2
```

---

# 10. 飽和型ボーナス

REN・BTB・Comboを無限に加点しない。

```text
RENBonus   = A*(1 - exp(-k*REN))
BTBBonus   = B*(1 - exp(-k2*BTB))
ComboBonus = C*(1 - exp(-k3*Combo))
```

---

# 11. T-spin地形の非線形評価

T-slotが多ければ常に良いわけではない。

```text
TSpinTerrainQuality =
    exp(-((T - T_opt)^2)/(2*sigma^2))
```

理想量付近を最大にする。

---

# 12. 重要な交互作用

## T-spin系

```text
TSpin * TSpinTerrain
TSpinTerrain * FutureFit
TSpin * BTB
TSpin * REN
```

## Tetris系

```text
Tetris * WellQuality
Tetris * BTB
WellQuality * FutureFit
```

## 穴系

```text
Hole * HoleDepth
Hole * HoleSpread
HoleSpread * FutureFit
```

## 盤面系

```text
Height * Bumpiness
MaxHeight * Hole
Overhang * Hole
```

## 配置系

```text
Placement * FutureFit
Placement * NextFit
Placement * WellQuality
```

---

# 13. Pure Single / Double / Triple

「1〜3ライン消しを減らす」をそのまま強い減点にすると、REN開始や盤面整理まで阻害する。

したがって、

```text
PureSinglePenalty
PureDoublePenalty
PureTriplePenalty
```

とし、価値のある消去を交互作用で救済する。

例えば、

```text
PureSinglePenalty
=
Single
*
(1 - RENContinuation)
*
(1 - FutureValue)
```

つまり「意味のない単発消去」を強く減点し、

```text
Single → REN
```

のような戦略的Singleはあまり減点しない。

---

# 14. BoardValue と ActionValue を分離

## BoardValue

```text
BoardValue =
    Height
  + MaxHeight
  + Holes
  + HoleSpread
  + Bumpiness
  + WellQuality
  + TSpinTerrain
  + Overhang
  + FutureFit
```

## ActionValue

```text
ActionValue =
    TSpin
  + Tetris
  + REN
  + BTB
  + Combo
  + LineClear
  + Placement
  + PC
```

最終値:

```text
MoveScore =
    BoardValue(after_move)
  + ActionValue(move)
```

---

# 15. 未来状態を含める

未来を読む探索では、

```text
V(s) =
    R(s)
  + gamma*E[V(s1)]
  + gamma^2*E[V(s2)]
  + gamma^3*E[V(s3)]
```

とする。

これにより、「今は少し不利だが3手後にTetris/BTB/T-spinに繋がる手」を評価できる。

---

# 16. 最終推奨モデル

実用上は次のハイブリッドを第一候補とする。

```text
Score(X)
=
Σ wi*xi

+ Σ aij*xi*xj
+ Σ bijk*xi*xj*xk

- Σ pi*xi^2

- D_height
- D_hole
- D_overhang

+ RENBonus
+ BTBBonus
+ ComboBonus
+ PCBonus
+ WellBonus
```

さらに、

```text
WellBonus =
    A*exp(-((WellDepth-D_opt)^2)/(2*sigma^2))
```

```text
PCBonus = B*sigmoid(PCPotential)
```

などを追加する。

最終的には、

```text
Polynomial
+ Sigmoid
+ Exponential
+ Gaussian
+ Saturation
```

の混合モデルとする。

---

# 17. 自動係数最適化

係数を人間だけで決めず、

```text
Genetic Algorithm
Optuna
CMA-ES
```

で最適化する。

最適化対象:

```text
wi
aij
bijk
pi
H_safe
D_opt
sigma
gamma
REN saturation
BTB saturation
```

---

# 18. 評価関数比較実験

同じミノ列・同じ探索方式で比較する。

### Model A: 一次

```text
Σ wi*xi
```

### Model B: 二次

```text
Σ wi*xi + Σ aij*xi*xj
```

### Model C: 三次

```text
Model B + Σ bijk*xi*xj*xk
```

### Model D: 非線形ハイブリッド

```text
Polynomial
+ Sigmoid
+ Exponential
+ Gaussian
+ Saturation
```

さらに、

```text
同一ミノ列
+ 同一探索時間
+ 同一計算資源
```

で比較する。

---

# 19. 過学習対策

```text
Training   : Seed 1〜700
Validation : Seed 701〜850
Test       : Seed 851〜1000
```

テスト列には学習で使用していないミノ列を使う。

シナリオも、

- 通常7-bag
- 完全ランダム
- T-spin向け
- Tetris向け
- PC向け
- 苦しいミノ列

に分ける。

---

# 20. 1手ごとのログ

```json
{
  "piece": "T",
  "x": 5,
  "rotation": 1,
  "t_spin": 1,
  "t_spin_terrain": 0.83,
  "holes": 2,
  "hole_spread": 0.21,
  "placement_quality": 0.91,
  "tetris": 0,
  "pure_single": 0,
  "pure_double": 0,
  "pure_triple": 0,
  "ren": 4,
  "btb": 2,
  "combo": 4,
  "pc": 0,
  "aggregate_height": 18,
  "max_height": 5,
  "bumpiness": 7,
  "well_quality": 0.92,
  "overhang": 1,
  "future_fit": 0.87,
  "evaluation": 8.231
}
```

---

# 21. 最終Fitness

ゲーム全体の強さは、1手評価と分けて測定する。

```text
FinalFitness =
    A*Survival
  + B*Lines
  + C*Score
  + D*AttackLines
  + E*TSpin
  + F*Tetris
  + G*REN
  + H*BTB
  + I*PC

  - J*Holes
  - K*HoleSpread
  - L*Height
  - M*Bumpiness
  - N*Overhang
```

ここにも、

```text
TSpin*TSpinTerrain
Tetris*WellQuality*BTB
REN*Combo*FutureFit
```

などを追加する。

---

# 22. 実装優先順位

## Phase 1

1. T-spin検出
2. T-spin地形検出
3. 穴数
4. Hole Depth
5. Hole Spread
6. Placement Quality
7. Tetris
8. Pure Single
9. Pure Double
10. Pure Triple
11. REN
12. BTB

## Phase 2

13. Combo
14. Perfect Clear
15. Aggregate Height
16. Max Height
17. Bumpiness
18. Well Quality
19. Overhang
20. Future Fit

## Phase 3

```text
一次
→ 二次
→ 三次
→ 重要な高次項
→ Sigmoid
→ Exponential
→ Gaussian
→ Saturation
```

## Phase 4

```text
Genetic Algorithm
Optuna
CMA-ES
```

で係数を自動最適化する。

---

# 23. 最終目標

最終的には、

```text
現在盤面
    ↓
20項目以上の特徴量
    ↓
正規化
    ↓
一次項
    ↓
二次交互作用
    ↓
三次交互作用
    ↓
重要な高次項
    ↓
Sigmoid / Exponential / Gaussian
    ↓
Saturation
    ↓
Move Score
```

という評価系を構築する。

目的は、

> 「何ライン消したか」ではなく、「その手によって現在の盤面、T-spin地形、Tetris用井戸、REN、BTB、穴、配置品質、Next/Holdを含む将来性がどれだけ改善したか」を定量化すること。

この評価関数をBeam Search、Expectimax、MCTS、A*などの探索アルゴリズムで共通利用し、

**探索方式 × 評価関数 × パラメータ**

の最適な組み合わせを自動探索する。
