# Tetris AI 開幕・中盤テンプレ統合設計書

## 1. 目的

本AIは、現在採用している

* 25特徴量
* 線形評価関数
* GPU/CPUによる高速候補手評価
* Next / Holdを考慮した探索

を維持しながら、

1. 開幕で強力なテンプレを選択する
2. Next 5からテンプレの成立可能性を判断する
3. 中盤では固定手順に縛られない
4. 現在の地形からT-Spin・Donate・LSTなどの有利な形を探索する
5. テンプレを優先することで探索効率を上げる
6. テンプレ失敗時には通常の地形評価へ即座に復帰する

ことを目的とする。

---

# 2. 基本方針

## 2.1 開幕

開幕では盤面が空であるため、

```text
Next 5
+
Hold
+
7-Bag情報
```

から、比較的高精度にテンプレ候補を予測できる。

したがって、

```text
Next 5
↓
開幕テンプレ候補を列挙
↓
テンプレごとの成立可能性を評価
↓
期待火力・安全性・高さ・派生を評価
↓
最適テンプレを選択
↓
通常探索にSoft Bonusとして統合
```

という方式を採用する。

---

## 2.2 中盤

中盤では開幕と異なり、

```text
Next順
```

だけでは判断できない。

中盤の最重要情報は、

```text
現在の地形
```

である。

したがって中盤では、

```text
現在盤面
↓
地形特徴抽出
↓
成立可能なT-Spin地形探索
↓
Donate可能地形探索
↓
LST継続可能性探索
↓
削り可能性探索
↓
テンプレ候補生成
↓
通常探索と統合
```

という方式にする。

---

# 3. 最重要設計原則

## テンプレを「固定手順」にしない

悪い実装例：

```text
DT砲を選択
↓
Lを置く
↓
Sを置く
↓
Tを置く
↓
途中で盤面が危険
↓
それでもDT砲を続行
↓
死亡
```

良い実装：

```text
DT砲候補を選択
↓
現在の地形を評価
↓
DT砲に近づく候補手を優先
↓
通常探索も同時に実行
↓
危険になった場合
↓
DT砲優先度を下げる
↓
通常地形探索へ復帰
```

テンプレは、

```text
Hard Constraint
```

ではなく、

```text
Soft Strategic Guidance
```

として扱う。

---

# 4. システム全体構造

```text
                    GAME START
                        │
                        ▼
                  Next 5取得
                        │
                        ▼
              Opening Template Selector
                        │
          ┌─────────────┼─────────────┐
          ▼             ▼             ▼
         TSD           TD系           PC系
          │             │             │
          └─────────────┼─────────────┘
                        │
                        ▼
                  Opening Plan
                        │
                        ▼
──────────────── 通常探索 ────────────────
                        │
                        ▼
                 Board Feature Extract
                        │
                        ▼
               Midgame Terrain Scanner
                        │
        ┌───────────────┼───────────────┐
        ▼               ▼               ▼
      T-Spin          Donate           LST
        │               │               │
        └───────────────┼───────────────┘
                        │
                        ▼
                 Candidate Generation
                        │
                        ▼
              25 Feature Linear Eval
                        │
                        +
                        │
              Template / Terrain Bonus
                        │
                        ▼
                    BEST MOVE
```

---

# 5. 開幕テンプレの優先順位

shiwehi.comのテンプレ一覧および「強い人が使っている開幕テンプレ」の使用傾向を考慮すると、最初に実装するテンプレは以下とする。

強いプレイヤーの実戦使用例では、ガムシロ積みとはちみつ砲の使用頻度が高く、DT砲などを相手に応じて使い分ける傾向が確認できる。

---

## Tier S：最優先実装

### 1. 開幕TSD

```text
優先度: S
実装難易度: 低
柔軟性: 非常に高い
高さ: 低い
派生性: 非常に高い
```

特徴：

* 初巡から攻撃可能
* 盤面を低く維持しやすい
* 中盤T-Spin探索へ移行しやすい
* LSTや削りへ接続しやすい
* テンプレ失敗時の復旧が容易

AIの最初のテンプレとして最適。

---

### 2. ガムシロ積み

```text
優先度: S
実戦性: 非常に高い
火力: 高い
派生性: 高い
```

強いプレイヤーの使用傾向からも優先度が高い。

AIでは、

```text
TD系の高火力候補
```

として扱う。

ただし固定構築ではなく、

```text
TST
↓
TSD
```

へ到達可能な地形を評価する。

---

### 3. はちみつ砲

```text
優先度: S
実戦性: 非常に高い
火力: 高い
```

ガムシロ積みと並び、実戦使用例が多い。

Next 5との適合性が高い場合、

```text
ガムシロ
vs
はちみつ砲
```

を比較する。

---

# 6. Tier A 開幕テンプレ

## 4. 開幕DT砲

```text
優先度: A
火力: 非常に高い
高さリスク: 中
失敗時リスク: 中
```

TSDとTSTを連続して狙える。

ただしAIでは、

```text
Next順が悪い
```

場合に無理に狙わせない。

---

## 5. 開幕BT砲

```text
優先度: A
火力: 高
派生: 高
実装難易度: 中
```

DT系の重要な選択肢。

DT砲との比較が必要。

---

## 6. Perfect Clear系

対象：

```text
開幕パフェ積み
グレースシステム
ILSZパフェ
ジグソーPC
ILZO-1
```

ただしPCは、

```text
成功した場合の価値
```

は高いが、

```text
ミノ順依存性
```

が大きい。

したがって、

```text
Next 5 Compatibility
```

が非常に高い場合のみ優先する。

---

# 7. Tier B 開幕テンプレ

以下は第2段階で追加する。

```text
MKO積み
合掌TSDパフェ
Antifate TSD
アルバトロスSP
アルバトロスTSD
QT砲
PC-Spin
```

---

# 8. 開幕テンプレ選択アルゴリズム

各テンプレについて、

$$
OpeningScore =
C
+
A
+
F
+
S
-
H
-
R
$$

を計算する。

---

## C: Compatibility

Next 5との適合度。

$$
C \in [0,1]
$$

評価する内容：

```text
必要ミノがNext 5内に存在するか
必要ミノが早く到着するか
Holdを使用することで成立するか
左右反転で成立するか
7-Bag残りとの相性
```

---

## A: Expected Attack

期待火力。

```text
TSD
TST
Tetris
Perfect Clear
BtB
```

などをテンプレ単位で推定する。

---

## F: Follow-up

テンプレ終了後の派生。

重要な評価項目：

```text
LSTへ移行可能
T-Spin地形が残る
平積みへ移行可能
PCへ移行可能
BTB維持
```

---

## S: Safety

テンプレ構築中の安全性。

評価：

```text
最大高さ
危険高さ到達率
穴の発生
オーバーハング
失敗時の復旧性
```

---

## H: Height Cost

テンプレ構築による高さ増加。

---

## R: Recovery Risk

途中でテンプレを放棄した場合の盤面の悪さ。

---

# 9. 開幕探索の計算量

Next 5を単純に列挙すると、

$$
7^5 = 16807
$$

通り。

ただし実際には7-Bag制約がある。

テンプレ選択時には、

```text
全16807通りの探索
```

は必要ない。

現在のNext 5は1つだけ与えられているため、

```text
TemplateCount × NextLength
```

程度の比較でよい。

テンプレ数を \(T\)、

Next数を \(N=5\) とすると、

$$
O(TN)
$$

となる。

例えば、

```text
テンプレ数: 20
Next: 5
```

なら、

$$
O(20×5)=O(100)
$$

程度。

非常に軽量。

---

# 10. 中盤テンプレの基本方針

## 中盤ではテンプレを選ばない

中盤で、

```text
現在はLST積みモード
```

のように固定するのは危険。

代わりに、

```text
現在地形から
何が成立するか
```

を探索する。

---

# 11. 中盤で優先的に探索する地形

## Tier S

### 1. T-Spin Slot

最優先。

探索対象：

```text
TSD
TSS
TST
Mini
```

ただし単純にT穴を探すだけでは不十分。

評価する。

```text
発火可能
NextでTが来る
HoldにTがある
T後にBTB維持
T後の盤面
```

---

### 2. T-Spin Donate

非常に重要。

Donateは、下穴を意図的に塞いでT-Spinを行いBTBを維持する技術として扱われる。

AIでは、

```text
穴を発見
↓
通常ならPenalty
↓
DonateでT-Spin可能性探索
↓
可能ならPenaltyを緩和
```

とする。

重要なのは、

```text
すべての穴を悪い穴と判断しない
```

こと。

---

### 3. LST

中盤で非常に優先度が高い。

LST積みは、

```text
T-Spin
+
Tetris
```

を継続できる地形構造として扱う。

AIでは固定テンプレではなく、

```text
LST Pattern Match
```

として検出する。

---

### 4. 削り

削りは非常に重要。

削りの本質は、

```text
ライン消去を利用して
T-Spin可能な形を作る
```

ことである。

したがって、

```text
候補手
↓
ライン消去
↓
消去後地形
↓
T-slot生成
```

を評価する必要がある。

---

# 12. Tier A 中盤探索

```text
STSD
STMB Cave
Double Dagger
Imperial Cross
階段Donate
Cut Copy
Anchor Set
BT砲
DT砲
欄干
```

これらは、

```text
TemplateName Match
```

よりも、

```text
Geometry Match
```

として実装する。

---

# 13. 中盤地形探索の設計

盤面を、

```text
10列 × 高さ
```

のビットボードとして保持する。

例：

```text
ColumnMask[10]
```

または、

```text
RowMask[Height]
```

を使用する。

---

# 14. Terrain Pattern Detector

各テンプレは、

```rust
struct TerrainPattern {
    name: PatternName,

    required_cells: BitBoard,
    forbidden_cells: BitBoard,

    rotations: RotationMask,

    mirror_allowed: bool,

    priority: f32,
}
```

として持つ。

---

## Required Cells

必ず埋まっている必要があるセル。

---

## Forbidden Cells

空いている必要があるセル。

---

## Don't Care

どちらでも良いセル。

これにより、

```text
完全一致
```

ではなく、

```text
部分地形一致
```

を行う。

これが中盤AIでは重要。

---

# 15. Pattern Match計算

単純なビット演算を使用する。

$$
(Board \& RequiredMask) = RequiredMask
$$

かつ、

$$
(Board \& ForbiddenMask) = 0
$$

で判定する。

計算量は、

$$
O(1)
$$

に近い。

実際には、

```text
Pattern
×
X位置
×
Y位置
×
左右反転
```

を確認する。

---

# 16. 中盤テンプレ探索の計算量

テンプレパターン数を \(P\)、

盤面横幅を \(W=10\)、

探索高さを \(H\) とする。

単純なパターン探索は、

$$
O(PWH)
$$

となる。

例えば、

```text
Pattern: 50
Width: 10
Height: 20
```

なら、

$$
50 × 10 × 20
=
10000
$$

回程度。

ビット演算中心なら十分軽い。

---

# 17. 重要な最適化

全テンプレを毎手詳細探索しない。

以下の順番にする。

```text
Candidate Move
↓
高速地形特徴抽出
↓
T-slot存在？
↓ No
通常評価
↓ Yes
詳細Pattern Search
```

つまり、

```text
Cheap Filter
↓
Expensive Search
```

の2段階にする。

---

# 18. 推奨する探索順序

## Stage 1

非常に軽量。

```text
Hole Count
Height
Max Height
Surface
Bumpiness
T-slot候補数
Donate候補数
```

---

## Stage 2

Stage 1で可能性がある場合。

```text
TSD
TST
TSS
Donate
LST
STMB
削り
```

を探索。

---

## Stage 3

有望な候補のみ。

```text
Next
Hold
Depth Search
```

を追加。

---

# 19. 候補手探索との統合

現在のAIの候補手数を \(M\)、

通常特徴量数を \(F=25\) とする。

通常評価：

$$
O(MF)
$$

25特徴量は固定なので、

実質、

$$
O(M)
$$

として扱える。

---

# 20. テンプレ探索追加後

高速地形検出を加える。

$$
O(MF)
+
O(MPWH)
$$

しかし実際には、

```text
Stage 1で大部分を除外
```

する。

詳細探索対象候補を \(K\) とし、

$$
K \ll M
$$

なら、

$$
O(MF)
+
O(KPWH)
$$

となる。

---

# 21. 推奨する実際の値

例えば、

```text
候補手 M = 40
Pattern P = 30
詳細探索候補 K = 5
Width W = 10
Height H = 12
```

とする。

通常評価：

$$
40×25=1000
$$

詳細Pattern：

$$
5×30×10×12
=
18000
$$

ただしビット演算中心。

CPUでも十分高速。

---

# 22. さらに重要な最適化

## Patternごとに探索しない

逆インデックスを作る。

例えば、

```text
T-slot候補あり
→ TSD / TST / STSDを確認

3幅穴あり
→ STMB Cave / Donateを確認

LST骨格あり
→ LSTを確認
```

とする。

```text
Terrain Signature
↓
Candidate Pattern Group
↓
Detailed Match
```

にする。

---

# 23. Terrain Signature

盤面から、

```text
signature
```

を生成する。

例：

```text
HAS_T_SLOT
HAS_3_WIDE_CAVITY
HAS_2_WIDE_HOLE
HAS_LST_BASE
HAS_STAIR
HAS_OVERHANG
HAS_DONATE_SHAPE
```

ビットフラグ化する。

```rust
struct TerrainSignature(u64);
```

---

# 24. 計算量削減効果

全Patternを探索：

$$
O(P)
$$

Signature分類後：

$$
O(K)
$$

ただし、

$$
K \ll P
$$

を目標とする。

例えば、

```text
全Pattern: 100
↓
Signature Filter
↓
候補Pattern: 5
```

なら詳細探索を約20分の1にできる。

---

# 25. 中盤AIの最終評価式

線形評価器を維持する。

$$
LinearScore
=
\sum_{i=1}^{25}W_iF_i
$$

これに、

$$
TerrainBonus
$$

を追加。

$$
FinalScore
=
LinearScore
+
TerrainBonus
$$

TerrainBonusは、

```text
T-Spin Opportunity
Donate Opportunity
LST Continuation
B2B Preservation
Future T-slot
Recovery
```

などから構成する。

ただし、

```text
大量の非線形評価
```

にはしない。

---

# 26. 推奨方式

```text
FinalScore
=
25 Feature Linear Score
+
Template Progress
+
Terrain Opportunity
-
Danger Override
```

---

# 27. Danger Override

線形評価だけでは、

```text
T-Spin価値が高すぎる
↓
危険な高さまで積む
```

可能性がある。

そこで、

```text
Danger Override
```

を別途持つ。

例：

```text
MaxHeight <= 8
    Penalty = 0

MaxHeight 9～12
    Linear Penalty

MaxHeight >= 13
    Strong Penalty
```

これは特徴量を増やすのではなく、

```text
探索枝刈り
```

として使用することもできる。

---

# 28. 最終優先順位

## 開幕

### Priority 1

```text
開幕TSD
ガムシロ積み
はちみつ砲
```

### Priority 2

```text
開幕DT砲
開幕BT砲
```

### Priority 3

```text
開幕パフェ
グレースシステム
ジグソーPC
ILSZ PC
```

---

# 29. 中盤

### Priority 1

```text
TSD/TST/TSS地形
T-Spin Donate
削り
LST
```

### Priority 2

```text
STSD
STMB Cave
Double Dagger
階段Donate
Cut Copy
```

### Priority 3

```text
BT砲
DT砲
Imperial Cross
Anchor Set
欄干
```

---

# 30. 実装ロードマップ

## Phase 1

```text
Opening Template Selector
Next 5 Compatibility
TSD
DT砲
```

---

## Phase 2

```text
ガムシロ
はちみつ砲
BT砲
Template Abort
```

---

## Phase 3

```text
Terrain Signature
T-slot Detector
Donate Detector
```

---

## Phase 4

```text
LST Detector
削り Detector
STMB Detector
```

---

## Phase 5

```text
Pattern Database
JSON / Binary Pattern Format
Pattern Index
```

---

# 31. 最終結論

このAIでは、

```text
開幕
=
Next 5ベースの戦略選択
```

にする。

一方、

```text
中盤
=
現在の地形ベースの機会探索
```

にする。

最も重要なのは、

```text
テンプレを覚えさせる
```

ことではない。

重要なのは、

```text
現在の地形から
T-Spin
Donate
削り
LST
BTB継続
高火力地形
```

を発見する能力を持たせることである。

したがって中盤AIの中心は、

```text
Template Selector
```

ではなく、

```text
Terrain Opportunity Search Engine
```

とする。

最終構造は以下。

```text
OPENING
Next 5
    │
    ▼
Opening Template Selector
    │
    ▼
Soft Strategic Guidance

MIDGAME
Current Board
    │
    ▼
Terrain Signature
    │
    ▼
Opportunity Search
    │
    ├── T-Spin
    ├── Donate
    ├── LST
    ├── 削り
    ├── STMB
    └── Other Patterns
    │
    ▼
Candidate Priority

CORE
Candidate Moves
    │
    ▼
25 Feature Linear Evaluation
    │
    +
    │
Terrain Opportunity Bonus
    │
    ▼
Best Move
```

この方式であれば、現在の線形評価器を維持したままテンプレを追加できる。

またテンプレ数が100、200と増えても、

```text
Terrain Signature
↓
Pattern Group Filter
↓
Bitboard Match
```

による枝刈りを行うため、全テンプレを毎回詳細探索する必要がなく、計算量の増加を比較的小さく抑えられる。
