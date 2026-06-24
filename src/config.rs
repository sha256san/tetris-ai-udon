// AIの報酬や評価パラメータをまとめた設定ファイル

/// Heuristic (評価関数) 関連のパラメータ
pub mod heuristic {
    /// デフォルトの重み
    /// [max_height, avg_height, bumpiness, holes, blocks_above_holes, wells, cleared_1_3, cleared_4]
    pub const DEFAULT_WEIGHTS: [f32; 8] = [
        -4.00,  // max_height: 高さ8超のときペナルティ
        -1.50,  // avg_height: 高さ8超のときペナルティ
        -1.00,  // bumpiness: 平らさの優先度を下げる（火力を出しやすくする）
        -7.50,  // holes: 穴は極力避ける
        -2.00,  // blocks_above_holes: 穴の上のブロックも避ける
        -0.50,  // wells: 深い谷の減点を減らす
        0.01,   // cleared_1_3: 1〜3ライン消去（低評価にして4ライン消しを待たせる）
        100.00, // cleared_4: 4ライン消去（テトリス、極めて高い評価）
    ];

    /// Iミノをホールドしたときの評価値ボーナス
    pub const HOLD_I_BONUS: f32 = 8.0;

    /// 深い穴ボーナスのAI評価値への変換倍率 (well_bonus_score * MULTIPLIER)
    pub const WELL_BONUS_MULTIPLIER: f32 = 0.02;
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
    pub const WELL_BASE_SCORE_EDGE: u32 = 10;    // 1列目, 10列目 (index 0, 9)
    pub const WELL_BASE_SCORE_MIDDLE: u32 = 320;  // 2列目〜9列目 (index 1〜8, index 6以外)
    pub const WELL_BASE_SCORE_TARGET: u32 = 500;  // 7列目 (index 6)
}
