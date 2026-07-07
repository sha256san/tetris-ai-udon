// AIの報酬や評価パラメータをまとめた設定ファイル

/// Heuristic (評価関数) 関連のパラメータ
pub mod heuristic {
    /// デフォルトの重み
    /// [max_height, avg_height, bumpiness, holes, blocks_above_holes, wells, cleared_1_3, cleared_4]
    pub const DEFAULT_WEIGHTS: [f32; 9] = [
        -4.00,  // max_height: 高さ8超のときペナルティ
        -1.00,  // avg_height: 高さ8超のときペナルティ（高く積むことを許容する）
        -1.00,  // bumpiness: 平らさの優先度を下げる（火力を出しやすくする）
        -7.50,  // holes: 穴は極力避ける
        -2.00,  // blocks_above_holes: 穴の上のブロックも避ける
        0.10,   // wells: 深い谷を作ることを推奨（テトリス穴の維持）
        -10.00, // cleared_1_3: 1〜3ライン消去（ペナルティにして4ライン消しを待たせる）
        150.00, // cleared_4: 4ライン消去（テトリス、極めて高い評価）
        15.00,  // t_slots: T-spin用のスロット（TSlot）を評価（T-spinの促進）
    ];

    /// Iミノをホールドしたときの評価値ボーナス
    pub const HOLD_I_BONUS: f32 = 8.0;

    /// 深い穴ボーナスのAI評価値への変換倍率 (well_bonus_score * MULTIPLIER)
    pub const WELL_BONUS_MULTIPLIER: f32 = 0.02;

    /// 4〜7列目（index 3〜6）に穴（ブロックの下の空きスペース）が存在する場合の追加ペナルティ（評価値）
    pub const TARGET_HOLE_PENALTY: f32 = -150.0;

    /// 3マス以上の深い谷が2列以上ある場合のペナルティ（評価値）
    pub const MULTIPLE_WELLS_PENALTY: f32 = -100.0;

    /// 1マスの埋まった穴（サイズ1のhole）が1箇所存在することに対するペナルティ（評価値）
    pub const ABANDONED_HOLE_PENALTY: f32 = -30.0;

    /// 先読みシミュレーションにおける将来スコアの割引率
    pub const LOOKAHEAD_DISCOUNT_FACTOR: f32 = 0.7;
}

/// Reinforcement Learning (強化学習) 関連の報酬パラメータ
pub mod rl {
    /// 1ターン生存するごとの生存報酬
    pub const SURVIVAL_REWARD: f32 = 1.0;

    /// ゲームオーバー時のペナルティ（負の値）
    pub const GAME_OVER_PENALTY: f32 = -500.0;

    /// Iミノをホールドへ保管したときのボーナス
    pub const HOLD_I_BONUS: f32 = 5.0;

    /// ライン消去数ごとの報酬（0〜4ライン）
    pub const LINE_CLEAR_REWARDS: [f32; 5] = [
        0.0,   // 0ライン消去
        0.0,   // 1ライン消去
        3.0,   // 2ライン消去
        5.0,   // 3ライン消去
        800.0,  // 4ライン消去 (テトリス)
    ];

    /// 深い穴ボーナスのRL報酬への変換倍率 (well_bonus_score * MULTIPLIER)
    pub const WELL_BONUS_MULTIPLIER: f32 = 0.02;
}

/// ゲームスコア (Game Score) 関連のパラメータ
pub mod game {
    /// ライン消去数ごとの獲得スコア（0〜4ライン）
    pub const LINE_CLEAR_SCORES: [u32; 5] = [
        0,   // 0ライン
        1, // 1ライン
        10, // 2ライン
        30, // 3ライン
        2000, // 4ライン (テトリス)
    ];

    /// 深い穴ボーナスのベース点数
    pub const WELL_BASE_SCORE_EDGE: u32 = 10;    // 1, 10列目 (index 0, 9)
    pub const WELL_BASE_SCORE_MIDDLE: u32 = 320;  // 2, 3, 8, 9列目 (index 1, 2, 7, 8)
    pub const WELL_BASE_SCORE_TARGET: u32 = 500;  // 4〜7列目 (index 3〜6)

    /// T-spin による獲得スコア
    pub const TSPIN_0_SCORE: u32 = 400;  // T-spin Null (ライン消去なし)
    pub const TSPIN_1_SCORE: u32 = 800;  // T-spin Single
    pub const TSPIN_2_SCORE: u32 = 1200; // T-spin Double
    pub const TSPIN_3_SCORE: u32 = 1600; // T-spin Triple
}
