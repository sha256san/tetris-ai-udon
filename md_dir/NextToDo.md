# Tetris AI 開幕・中盤テンプレ統合 実装タスク一覧 (NextToDo.md)

本ドキュメントは、[`md_dir/addplan7.md`](file:///home/sha256san/tetris_ai/md_dir/addplan7.md) に基づき、現在の **25特徴量評価モデル・GPU高速探索** を維持しながら、**「Next 5適合型 開幕テンプレセレクター」** および **「地形シグネチャ連動型 中盤機会探索エンジン（T-Spin・Donate・LST・削り）」** を段階的に実装するための詳細タスクリストです。

---

## 🏗️ 全体設計方針 (Core Architecture)

```mermaid
flowchart TB
    Start[GAME START / TURN] --> CheckPhase{開幕 or 中盤?}

    subgraph OPENING [開幕フェーズ (lines_cleared < 8)]
        Next5[Next 5 + Hold + 7-Bag] --> Selector[Opening Template Selector]
        Selector --> Compatibility[適合度 C 判定 (O(TN))]
        Compatibility --> TopTemplate[最良テンプレ選択\n(開幕TSD/ガムシロ/はちみつ/DT/BT/PC)]
        TopTemplate --> SoftGuide[Soft Strategic Guidance 加算]
    end

    subgraph MIDGAME [中盤フェーズ (lines_cleared >= 8)]
        Board[現在盤面 (Board)] --> Signature[Terrain Signature (u64 ビットマスク)]
        Signature --> CheapFilter[Cheap Filter (候補グループ絞り込み)]
        CheapFilter --> OppSearch[Opportunity Search Engine]
        OppSearch --> Opp1[T-Spin Slot (TSD/TST/TSS)]
        OppSearch --> Opp2[T-Spin Donate (下穴一時塞ぎ)]
        OppSearch --> Opp3[LST 積み継続構造]
        OppSearch --> Opp4[削り (消去後Tスロット化)]
        OppSearch --> Opp5[STMB Cave / STSD / 階段]
        Opp1 & Opp2 & Opp3 & Opp4 & Opp5 --> TerrainBonus[Terrain Opportunity Bonus]
    end

    CheckPhase -->|開幕| OPENING
    CheckPhase -->|中盤| MIDGAME

    SoftGuide --> CoreEval[Candidate Moves Generation]
    TerrainBonus --> CoreEval

    subgraph EVAL [評価 & 決定]
        CoreEval --> Linear25[25 Feature Linear / GPU Evaluation]
        Linear25 --> DangerCheck{Danger Override\n(MaxHeight >= 13?)}
        DangerCheck -->|危険| EmergencyDefense[テンプレ破棄 & 防御・下層維持]
        DangerCheck -->|安全| FinalScore[Final Score 合成]
        EmergencyDefense --> FinalScore
        FinalScore --> BestMove[BEST MOVE 決定]
    end
```

---

## 📋 フェーズ別 実装ToDoリスト

### 🚀 Phase 1: 開幕テンプレセレクター & Next 5 適合度判定
> **目的**: 初巡〜2巡目の Next 5 + Hold から最適な開幕テンプレを $O(TN)$ で高速判定し、Soft Strategic Guidance（軟弱バイアス）として通常探索に統合する。

- [ ] **1.1 開幕テンプレ定義と適合度計算モジュール (`src/opening_selector.rs`) の新設**
  - [ ] Next 5 + Hold + 7-Bag 順序適合度スコア $C \in [0, 1]$ 計算関数の実装
  - [ ] 左右反転（Mirroring）の自動判定と最適ミラーの選択
  - [ ] 総合スコア式の実装: $OpeningScore = C + A(\text{期待火力}) + F(\text{派生性}) + S(\text{安全性}) - H(\text{高さ}) - R(\text{リスク})$
- [ ] **1.2 Tier S 開幕テンプレの登録**
  - [ ] **開幕TSD (Opening TSD)**: 早期発火・低層維持・LST移行率 No.1
  - [ ] **ガムシロ積み (Gammushiro / TD系)**: TST $\rightarrow$ TSD 連続高火力
  - [ ] **はちみつ砲 (Hachimitsu Cannon)**: TSD $\rightarrow$ TST $\rightarrow$ Tetris 高速展開
- [ ] **1.3 Tier A 開幕テンプレの登録**
  - [ ] **開幕DT砲 (Opening DT Cannon)**: TSD $\rightarrow$ TST 確定2連打
  - [ ] **開幕BT砲 (Opening BT Cannon)**: TST $\rightarrow$ TSD / C-Spin 派生
  - [ ] **Perfect Clear 系**: 開幕パフェ、グレースシステム、ILSZ PC（適合度 $C > 0.9$ のみ発火）
- [ ] **1.4 Soft Guidance 評価結合 & 自動離脱 (Template Abort)**
  - [ ] 固定配置の強制（Hard Script）を廃止し、テンプレ形状に近づく候補手に `+OpeningFitScore` を付与
  - [ ] 相手の邪魔やミノ順狂いで危険高度（高さ > 8）に達した場合、即座にテンプレを破棄して通常地形評価へ復帰

---

### 🔍 Phase 2: 中盤地形シグネチャ (Terrain Signature) & 高速ビットボード照合
> **目的**: 中盤において、毎手全テンプレを網羅探索することなく、$O(1)$ ビット演算で地形特徴を抽出し、探索候補グループを絞り込む。

- [ ] **2.1 ビットボード構造 (`src/bitboard.rs`) の定義**
  - [ ] `10列 × 20行` 盤面のビットボード表現 (`[u32; 10]` または `[u16; 20]`)
  - [ ] 高速ビット演算による行消去・接地シミュレーション
- [ ] **2.2 `TerrainSignature(u64)` ビットフラグエンジンの実装**
  - [ ] `HAS_T_SLOT`: T字スロット・屋根構造の存在フラグ
  - [ ] `HAS_3_WIDE_CAVITY`: 3マス幅の凹み（STMB Cave / ドネイト可能領域）
  - [ ] `HAS_2_WIDE_HOLE`: 2マス幅の溝（TSD / STSD 構築領域）
  - [ ] `HAS_LST_BASE`: 1列井戸＋中央平坦（LST積み基盤）
  - [ ] `HAS_STAIR`: 階段形状（階段ドネイト・Parapet）
  - [ ] `HAS_OVERHANG`: 意図的なオーバーハング（屋根）
  - [ ] `HAS_BURIED_HOLE`: 既存下穴（ドネイト評価連携用）
- [ ] **2.3 2段階フィルタリング パイプラインの実装**
  - [ ] **Stage 1 (Cheap Filter)**: 地形シグネチャにより対象外パターンを即時除外（$100 \rightarrow 5$ パターンへ圧縮）
  - [ ] **Stage 2 (Detailed Match)**: 有望パターンのみ $O(K)$ でビットマスク部分一致判定 (`(Board & Req) == Req && (Board & Forbid) == 0`)

---

### ⚡ Phase 3: 中盤 Tier S 機会探索エンジン (T-Spin・Donate・LST・削り)
> **目的**: 盤面から「T-Spin発火」「下穴ドネイト」「LST維持」「削り（消去後Tスロット化）」の有利な手を自律発見する。

- [ ] **3.1 T-Spin Slot 機会探索 (T-Spin Opportunity)**
  - [ ] TSD / TST / TSS / Mini の即時発火・1手前準備形状の検出
  - [ ] Next 5 / Hold にTミノがあるかとの時間的同期評価
- [ ] **3.2 T-Spin Donate (下穴一時塞ぎドネイト) 探索**
  - [ ] 盤面の穴を「悪手穴」と「戦術的ドネイト屋根」に明確に分離
  - [ ] ドネイト後にT-Spinを発火して下穴を回収できる場合、穴ペナルティを大幅緩和＋大加点
  - [ ] 下穴が深く埋まりすぎる「泥沼ドネイト」は `BURIED_HOLE_DONATION_PENALTY` で抑止
- [ ] **3.3 LST 積み継続性判定 (LST Structure)**
  - [ ] 0列または9列にテトリス用井戸を空け、反対側で L, S, T を組む LST 骨格の認識
  - [ ] LST 継続可能手に対して `LST_CONTINUATION_BONUS` を付与し、長期BTB火力を維持
- [ ] **3.4 削り (Kezuri / Line Clearing for T-Slot) 探索**
  - [ ] 候補手を置いた際の 1〜2 ライン消去によって、消去後の盤面に新たな T-Slot が出現するかを先読み評価
  - [ ] 「通常消去ペナルティ」を上書き免除し、「削りによるT-Spin誘爆手」として高評価

---

### 🏰 Phase 4: 中盤 Tier A/B パターン拡充 & Danger Override (危険高度防御)
> **目的**: 複合戦術パターンの認識強化と、盤面が高くなった際の厳格な緊急防御モードの確立。

- [ ] **4.1 Tier A/B 中盤幾何パターンの実装**
  - [ ] **STSD (Single-Triple Super Drop / Continuous TSD)**
  - [ ] **STMB Cave (S/T/Z/J/L による TSD 洞窟)**
  - [ ] **Double Dagger / Imperial Cross (交差型 T-Spin 複合体)**
  - [ ] **階段ドネイト / Parapet / Cut Copy / Anchor Set**
- [ ] **4.2 Danger Override (危険高度急増・緊急防御システム)**
  - [ ] 最大高さ $h_{\max} \le 8$: テンプレ・T-Spin・ドネイトを積極推進（通常評価）
  - [ ] 最大高さ $h_{\max} \in [9, 12]$: テンプレボーナスを徐々に減衰させ、平坦化・消去を優先
  - [ ] 最大高さ $h_{\max} \ge 13$: **テンプレ優先度を完全剥奪（0点化）**、`HeightRisk (-75.0)` を最優先し即座にダウンスタック・消去で低層（<=6段）へ戻す

---

### 🧪 Phase 5: パターンデータベース・学習連携 & 検証
> **目的**: パターン定義を整理し、25特徴量の自己対戦並列学習（2ワーカー）との整合性を検証する。

- [ ] **5.1 テンプレ・パターン定義の管理 (`templates/` / `src/patterns.rs`)**
  - [ ] 各開幕テンプレおよび中盤幾何パターンの定義を構造化
- [ ] **5.2 ユニットテスト・ベンチマークの作成**
  - [ ] `test_opening_selector_next5_compatibility`: Next順に応じたガムシロ/はちみつ/TSDの選択検証
  - [ ] `test_terrain_signature_fast_filter`: ビットボードシグネチャによる高速枝刈りの正当性検証
  - [ ] `test_midgame_donate_and_kezuri_detection`: ドネイトおよび削り手へのボーナス付与検証
  - [ ] `test_danger_override_abort`: 高さ13段以上でのテンプレ自動破棄・安全防御への切り替え検証
- [ ] **5.3 2ワーカー並列学習 (`./start_training_daemon.sh 100 5`) での動作確認**
  - [ ] テンプレバイアスと25特徴量の重み学習が安定して収束することを確認

---

## 📅 推奨実装ステップ

1. **Step 1**: Phase 1 (開幕TSD・ガムシロ・はちみつ・DT砲の Next 5 適合判定 & Soft Guidance)
2. **Step 2**: Phase 2 (ビットボード & `TerrainSignature(u64)` 高速判定)
3. **Step 3**: Phase 3 (中盤 T-Spin・ドネイト・LST・削りの機会探索エンジン)
4. **Step 4**: Phase 4 (Danger Override & Tier A複合パターンの幾何照合)
5. **Step 5**: Phase 5 (ユニットテスト、ベンチマーク、GitHub Commit & Push)
