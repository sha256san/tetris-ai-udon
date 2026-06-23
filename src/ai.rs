use crate::tetris::{Game, Piece, Board, BlockType, BOARD_WIDTH, INTERNAL_HEIGHT};
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModel {
    pub weights: Vec<f32>, // 特徴量に対応する重み
}

impl AiModel {
    pub fn new_default() -> Self {
        // 合計8つの特徴量に対するデフォルト重み
        // [max_height, avg_height, bumpiness, holes, blocks_above_holes, wells, cleared_1_3, cleared_4]
        AiModel {
            weights: vec![
                -4.00, // max_height: 高さ8超のときペナルティ
                -1.50, // avg_height: 高さ8超のときペナルティ
                -1.00, // bumpiness: 平らさの優先度を下げる（火力を出しやすくする）
                -7.50, // holes: 穴は極力避ける
                -2.00, // blocks_above_holes: 穴の上のブロックも避ける
                -0.50, // wells: 深い谷の減点を減らす
                0.01,  // cleared_1_3: 1〜3ライン消去（低評価にして4ライン消しを待たせる）
                400.00, // cleared_4: 4ライン消去（テトリス、極めて高い評価）
            ],
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
pub fn enumerate_all_moves(game: &Game, model: &AiModel) -> Vec<CandidateMove> {
    let mut moves = Vec::new();

    // 1. ホールドを使わない場合
    enumerate_moves_for_piece(game, game.current_piece.block_type, false, model, &mut moves);

    // 2. ホールドを使う場合
    if !game.hold_locked {
        let next_piece_type = match game.hold_piece {
            Some(held) => held,
            None => {
                // ホールドが空なら、Nextキューの最初のミノになる
                game.bag.peek_next(1)[0]
            }
        };
        enumerate_moves_for_piece(game, next_piece_type, true, model, &mut moves);
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
                
                // 特徴量抽出
                let features = extract_features(&temp_board_after_clear, cleared);
                let mut eval_score = model.evaluate(&features);

                // Iミノをホールドに入れる行動（現在Iミノを手放してホールドする）に加点
                // ホールド操作かつ、「ホールドに入るのがIミノ」= 現在の手がcurrent_pieceをIミノとしてホールドした場合
                if use_hold && game.current_piece.block_type == BlockType::I && game.hold_piece != Some(BlockType::I) {
                    // Iミノをホールドに「保管」する行動：4ライン消し準備ボーナス
                    eval_score += 8.0;
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
