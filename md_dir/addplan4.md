# 実戦特化 T-Spin Mini 最適化・BTB継続検証・操作速度高速化 計画書 (addplan4)

## 目的

1. **T-Spin Mini 後の BTB 継続検証 & 追跡ログ機能**:
   - T-Spin Mini 単体では火力（0〜1段）が低いため、Mini 発火後に **BTB（Back-to-Back）を維持して本命の Tetris (4-line) や Full T-Spin (TSD / TST) に繋げられているか** を自動判定・記録する。
   - BTB を活かせずに途切れた Mini を「無駄打ち（Wasted Mini）」として検知し、ペナルティで抑止する。
2. **T-Spin 直前の1手（Turn -1: 屋根・スロット仕込み手）のログ強調表示**:
   - ログ（JSON / TXT）において、T-Spin 発火の直前1手 (`relative_turn == -1`) を視覚的に強調し、AIがどのミノで屋根や土台をセットアップしたかを明確化する。
3. **実戦操作の高速化（ハードドロップ優先 & スピン後の即時ロック）**:
   - **ハードドロップ優先**: ソフトドロップを使わずに上空から直接落とせる位置には、ソフトドロップを介さず直接ハードドロップで配置する。
   - **スピン・潜り込み完了後の即時ハードドロップ**: ソフトドロップや回転入れ（SRSキック）で目標地点に到達した直後、ロックディレイタイマーの待機時間を排除するため、最後の操作として即座にハードドロップ（HardDrop）を発行して着地・固定する。

---

# Phase 1: T-Spin Mini 後の BTB 継続判定 & 追跡ログ機能

## タスク一覧 (Tasks 001 - 025)

- [ ] **Task 001**: `src/tspin_recorder.rs` の `TSpinEventRecord` に `btb_continued_after: bool` および `next_heavy_attack: Option<String>` フィールドを追加。
- [ ] **Task 002**: T-Spin Mini 発火後の「直後5ターン (`history_after_5`)」において、BTB が継続した状態で Tetris または TSD/TST が発火したかを検証する判定ロジックを実装。
- [ ] **Task 003**: Mini 後に BTB が途切れた（Single/Double 等で消費または何も打たずにゲームオーバー）場合、`btb_status = "Wasted Mini (BTB Broken)"` と判定。
- [ ] **Task 004**: Mini 後に Tetris または TSD/TST に繋がった場合、`btb_status = "Successful Mini -> Heavy Attack (BTB Maintained)"` と判定。
- [ ] **Task 005**: TXT ログのヘッダーに BTB 継続成否および次発火アクションを分かりやすくサマリー表示。
- [ ] **Task 006**: 単体テスト `test_tspin_mini_btb_continuation_tracking` を作成し、Mini 後の BTB 追跡が正しく記録されることを検証。

---

# Phase 2: T-Spin 直前の1手 (Turn -1) のログ強調表示

## タスク一覧 (Tasks 026 - 050)

- [ ] **Task 026**: `src/tspin_recorder.rs` のテキスト出力ロジックを改修し、`relative_turn == -1` のスナップショットを以下のフォーマットで強調表示：
  ```text
  ================================================================================
  🔥🔥🔥 【T-SPIN SETUP MOVE: 1 TURN BEFORE TRIGGER (Turn X)】 🔥🔥🔥
  Placement Piece: T-Slot Roof / Foundation Placement
  ================================================================================
  ```
- [ ] **Task 027**: 直前の手で配置されたミノの位置を ASCII 盤面上で強調マーク（`*` または `[X]` 記号）でハイライト。
- [ ] **Task 028**: JSON ログに `is_setup_turn: true` フラグを付与。
- [ ] **Task 029**: 単体テスト `test_setup_turn_highlighting` を作成し、直前の手情報が強調出力されることを確認。

---

# Phase 3: 操作最適化 ① ハードドロップ優先ロジック

## タスク一覧 (Tasks 051 - 075)

- [ ] **Task 051**: `src/ai.rs` の `search_reachable_landings` において、上空から真下に落とせるオープンな着地点（天井・屋根に遮られていない着地点）に対して、経路 `path` を `[MoveLeft/Right/Rotate, HardDrop]` の最小ステップで構成。
- [ ] **Task 052**: 不要なソフトドロップ（SoftDrop）の連打を排除し、上空直通可能な手は常にハードドロップで配置。
- [ ] **Task 053**: 単体テスト `test_hard_drop_preference_for_open_landings` を作成。

---

# Phase 4: 操作最適化 ② スピン・ソフトドロップ後の即時ハードドロップ

## タスク一覧 (Tasks 076 - 100)

- [ ] **Task 076**: スピン入れ（Tuck / Kick）やソフトドロップが必要な着地手について、最終目標セルに到達した時点で末尾に必ず `MoveAction::HardDrop` を追加。
- [ ] **Task 077**: 実戦で接地判定待ち（ロックディレイ0.5秒）を発生させず、目標地点進入と同時に即座にロックを完了させる。
- [ ] **Task 078**: `MoveAction` シーケンスの末尾が必ず `HardDrop` で終了することを保証。
- [ ] **Task 079**: 単体テスト `test_spin_path_ends_with_immediate_hard_drop` を作成。

---

# Phase 5: T-Spin Mini 無駄打ちペナルティ & 評価関数の調整

## タスク一覧 (Tasks 101 - 120)

- [ ] **Task 101**: `src/config.rs` に `WASTED_TSPIN_MINI_PENALTY: f32 = -30.0;` を追加。
- [ ] **Task 102**: `src/ai.rs` の `extract_20_features` において、BTB 継続に繋がらない単発 T-Spin Mini の評価値を引き下げ、TSD / Tetris への繋ぎとして機能する場合のみ許容。
- [ ] **Task 103**: 単体テスト `test_wasted_mini_suppression` を作成。

---

# Phase 6: 実機ベンチマーク・テスト・総合検証

## タスク一覧 (Tasks 121 - 140)

- [ ] **Task 121**: `cargo test` で全テストが合格することを確認。
- [ ] **Task 122**: AI 自動プレイおよびベンチマークを実行し、T-Spin Mini 後の BTB 継続ログおよび直前1手強調ログが出力されることを確認。
- [ ] **Task 123**: `walkthrough.md` に結果をまとめる。
