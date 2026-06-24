use crate::tetris::{Game, Piece, Board, BlockType, BOARD_WIDTH, INTERNAL_HEIGHT, get_well_bonus};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModel {
    pub weights: Vec<f32>, // 特徴量に対応する重み
}

impl AiModel {
    pub fn new_default() -> Self {
        AiModel {
            weights: crate::config::heuristic::DEFAULT_WEIGHTS.to_vec(),
        }
    }

    // 評価値を計算（高いほど良い）
    pub fn evaluate(&self, features: &[f32]) -> f32 {
        let mut score = 0.0;
        for i in 0..self.weights.len() {
            score += self.weights[i] * features[i];
        }
        score
    }
}

#[derive(Debug, Clone)]
pub struct CandidateMove {
    pub x: i32,
    pub rotation: usize,
    pub use_hold: bool,
    pub features: Vec<f32>,
    pub eval_score: f32,
    pub final_piece: Piece,
}

// 盤面の特徴量を抽出する
pub fn extract_features(board: &Board, cleared_lines: usize) -> Vec<f32> {
    let mut heights = [0; BOARD_WIDTH];
    
    // 各列の高さを計算
    for x in 0..BOARD_WIDTH {
        let mut height = 0;
        for y in 0..INTERNAL_HEIGHT {
            if board[y][x].is_some() {
                height = INTERNAL_HEIGHT - y;
                break;
            }
        }
        heights[x] = height as i32;
    }

    // 1. 最大高さ（最大高さが8以下のときはペナルティをなくす）
    let raw_max_height = *heights.iter().max().unwrap_or(&0) as f32;
    let max_height = if raw_max_height <= 8.0 { 0.0 } else { raw_max_height };

    // 2. 平均高さ（最大高さが8以下のときはペナルティをなくす）
    let avg_height = if raw_max_height <= 8.0 {
        0.0
    } else {
        (heights.iter().sum::<i32>() as f32) / (BOARD_WIDTH as f32)
    };

    // 3. 高低差の合計 (Bumpiness)
    let mut bumpiness = 0;
    for x in 0..(BOARD_WIDTH - 1) {
        bumpiness += (heights[x] - heights[x + 1]).abs();
    }
    let bumpiness = bumpiness as f32;

    // 4. 穴の数、および 5. 穴の上のブロック数
    let mut holes = 0;
    let mut blocks_above_holes = 0;
    for x in 0..BOARD_WIDTH {
        let mut block_found = false;
        let mut block_count_above_hole = 0;
        for y in 0..INTERNAL_HEIGHT {
            if board[y][x].is_some() {
                block_found = true;
                block_count_above_hole += 1;
            } else if block_found {
                // ブロックが見つかった後に空のマスがある＝穴
                holes += 1;
                blocks_above_holes += block_count_above_hole;
            }
        }
    }

    // 6. 谷の深さの合計 (Wells)
    let mut wells_depth = 0;
    for x in 0..BOARD_WIDTH {
        let left = if x == 0 { INTERNAL_HEIGHT as i32 } else { heights[x - 1] };
        let right = if x == BOARD_WIDTH - 1 { INTERNAL_HEIGHT as i32 } else { heights[x + 1] };
        let h = heights[x];
        let diff = std::cmp::min(left, right) - h;
        if diff > 0 {
            wells_depth += diff;
        }
    }

    let cleared_1_3 = if cleared_lines < 4 { cleared_lines as f32 } else { 0.0 };
    let cleared_4 = if cleared_lines == 4 { 1.0 } else { 0.0 };

    vec![
        max_height,
        avg_height,
        bumpiness,
        holes as f32,
        blocks_above_holes as f32,
        wells_depth as f32,
        cleared_1_3,
        cleared_4,
    ]
}

// すべての可能な配置（候補手）を列挙する
// opening_turn: オープニングシーケンスの何手目か（0-indexed）
pub fn enumerate_all_moves(
    game: &Game,
    model: &AiModel,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
) -> Vec<CandidateMove> {
    let mut moves = Vec::new();

    // 1. ホールドを使わない場合
    enumerate_moves_for_piece(game, game.current_piece.block_type, false, model, opening, opening_turn, &mut moves);

    // 2. ホールドを使う場合
    if !game.hold_locked {
        let next_piece_type = match game.hold_piece {
            Some(held) => held,
            None => {
                // ホールドが空なら、Nextキューの最初のミノになる
                game.bag.peek_next(1)[0]
            }
        };
        enumerate_moves_for_piece(game, next_piece_type, true, model, opening, opening_turn, &mut moves);
    }

    // 評価スコアの高い順にソート
    moves.sort_by(|a, b| b.eval_score.partial_cmp(&a.eval_score).unwrap_or(std::cmp::Ordering::Equal));
    moves
}

// 特定のミノ種について、配置候補を全探索して moves に追加
fn enumerate_moves_for_piece(
    game: &Game,
    block_type: BlockType,
    use_hold: bool,
    model: &AiModel,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
    moves: &mut Vec<CandidateMove>,
) {
    let spawn_x = match block_type {
        BlockType::I => 3,
        BlockType::O => 4,
        _ => 3,
    };

    // 回転状態 0〜3 を試す
    for rotation in 0..4 {
        if block_type == BlockType::O && rotation > 0 {
            continue; // Oミノは回転探索をスキップ
        }

        // x 座標の走査範囲。ミノのブロック相対座標の最大最小を求めて、盤面外に出ないようにする
        let offsets = crate::tetris::get_piece_offsets(block_type, rotation);
        let min_dx = offsets.iter().map(|&(dx, _)| dx).min().unwrap();
        let max_dx = offsets.iter().map(|&(dx, _)| dx).max().unwrap();

        let start_x = -min_dx;
        let end_x = BOARD_WIDTH as i32 - max_dx;

        for target_x in start_x..end_x {
            // スポーン位置 (spawn_x) から target_x への経路が存在するかチェック
            // 簡易的に、Y=2（出現高さ）において、spawn_x から target_x までの水平経路に衝突がないかを判定
            let step = if target_x > spawn_x { 1 } else { -1 };
            let mut path_ok = true;
            let mut curr_x = spawn_x;
            
            // 出現位置で既に衝突しているか？
            let spawn_piece = Piece { block_type, x: spawn_x, y: 2, rotation };
            if !game.is_valid_position(&spawn_piece) {
                path_ok = false;
            }

            if path_ok {
                while curr_x != target_x {
                    curr_x += step;
                    let test_piece = Piece { block_type, x: curr_x, y: 2, rotation };
                    if !game.is_valid_position(&test_piece) {
                        path_ok = false;
                        break;
                    }
                }
            }

            if !path_ok {
                continue;
            }

            // ハードドロップ位置をシミュレート
            let mut test_piece = Piece { block_type, x: target_x, y: 2, rotation };
            while game.is_valid_position(&Piece {
                block_type,
                x: test_piece.x,
                y: test_piece.y + 1,
                rotation,
            }) {
                test_piece.y += 1;
            }

            // 盤面に固定したと仮定した仮想盤面を作成
            let mut temp_board = game.board;
            let mut cells_locked_count = 0;
            for &(cx, cy) in &test_piece.get_cells() {
                if cx >= 0 && cx < BOARD_WIDTH as i32 && cy >= 0 && cy < INTERNAL_HEIGHT as i32 {
                    temp_board[cy as usize][cx as usize] = Some(block_type);
                    cells_locked_count += 1;
                }
            }

            // すべてのセルが正しく配置された場合のみ評価
            if cells_locked_count == 4 {
                // ライン消去数をシミュレート
                let (temp_board_after_clear, cleared) = simulate_line_clears(&temp_board);

                // オープニングフェーズが有効かチェック
                let is_opening_active = opening
                    .map_or(false, |o| game.lines_cleared < o.active_until_lines);

                // 特徴量抽出
                let features = extract_features(&temp_board_after_clear, cleared);

                // オープニング中は opening_weights を使用、それ以外は model.weights
                let mut eval_score = if is_opening_active {
                    let o = opening.unwrap();
                    let mut s = 0.0f32;
                    for i in 0..o.opening_weights.len().min(features.len()) {
                        s += o.opening_weights[i] * features[i];
                    }
                    s
                } else {
                    model.evaluate(&features)
                };

                // オープニング適合ボーナス（目標盤面への近さ）
                if is_opening_active {
                    // 1) 列高さ目標への近さボーナス
                    eval_score += crate::opening::evaluate_opening_fit(
                        &temp_board_after_clear,
                        opening.unwrap(),
                        game,
                        opening_turn,
                    );
                    // 2) シーケンス配置一致ボーナス
                    eval_score += crate::opening::evaluate_sequence_match(
                        opening.unwrap(),
                        game,
                        opening_turn,
                        block_type,
                        target_x,
                        rotation,
                    );
                }

                // 縦3マス以上の深い穴が1列しかない場合のボーナス（通常フェーズのみ）
                if !is_opening_active {
                    let well_bonus_score = get_well_bonus(&temp_board_after_clear);
                    if well_bonus_score > 0 {
                        let ai_bonus = (well_bonus_score as f32) * crate::config::heuristic::WELL_BONUS_MULTIPLIER;
                        eval_score += ai_bonus;
                    }
                }

                // Iミノをホールドに入れる行動に加点
                if use_hold && game.current_piece.block_type == BlockType::I && game.hold_piece != Some(BlockType::I) {
                    eval_score += crate::config::heuristic::HOLD_I_BONUS;
                }

                moves.push(CandidateMove {
                    x: target_x,
                    rotation,
                    use_hold,
                    features,
                    eval_score,
                    final_piece: test_piece,
                });
            }
        }
    }
}

// 仮想的にライン消去を行い、消去後の盤面と消去ライン数を返す
fn simulate_line_clears(board: &Board) -> (Board, usize) {
    let mut cleared = 0;
    let mut new_board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
    let mut target_y = INTERNAL_HEIGHT - 1;

    for y in (0..INTERNAL_HEIGHT).rev() {
        let mut is_full = true;
        for x in 0..BOARD_WIDTH {
            if board[y][x].is_none() {
                is_full = false;
                break;
            }
        }

        if is_full {
            cleared += 1;
        } else {
            new_board[target_y] = board[y];
            if target_y > 0 {
                target_y -= 1;
            }
        }
    }
    (new_board, cleared)
}

/// AIのロジックに基づいて、現在のピース及びNextキュー内のすべてのピースの将来の配置位置をシミュレートする
pub fn simulate_future_moves(
    game: &Game,
    model: &AiModel,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
) -> Vec<(Piece, BlockType)> {
    let mut temp_game = game.clone();
    let mut results = Vec::new();
    let mut curr_turn = opening_turn;

    // Nextキューの長さ（通常5個）
    let num_nexts = game.bag.peek_next(5).len();

    // 現在の手も含めて、Nextにあるピースの数だけ先をシミュレート
    for _ in 0..=num_nexts {
        if temp_game.game_over {
            break;
        }

        let candidates = enumerate_all_moves(&temp_game, model, opening, curr_turn);
        if candidates.is_empty() {
            break;
        }

        let best_move = &candidates[0];

        // 予測された配置ピースを記録
        let placed_piece = best_move.final_piece.clone();
        let bt = placed_piece.block_type;
        results.push((placed_piece, bt));

        // 仮想ゲーム状態の更新
        if best_move.use_hold {
            temp_game.hold();
        }
        
        // ピースを決定された位置に設定
        temp_game.current_piece.x = best_move.final_piece.x;
        temp_game.current_piece.rotation = best_move.final_piece.rotation;
        
        // 固定して次のピースへ進める
        temp_game.hard_drop();

        // オープニングターンカウンタの更新
        if let Some(op) = opening {
            let max_turns = if let Some(branch) = op.get_active_branch(&temp_game) {
                branch.parsed_placements.len()
            } else {
                op.parsed_placements.len()
            };
            if temp_game.lines_cleared < op.active_until_lines && curr_turn < max_turns {
                curr_turn += 1;
            }
        }
    }

    results
}
