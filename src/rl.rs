use crate::tetris::{Game, BlockType, get_well_bonus, Piece};
use crate::ai::{AiModel, enumerate_all_moves, CandidateMove};
use rand::Rng;

// 1エピソード（1回のゲームオーバーまで）を自己対戦でプレイしながら学習する
// 戻り値: (消去ライン数, ターン数, 累積報酬)
pub fn run_rl_episode(
    model: &mut AiModel,
    epsilon: f32,
    alpha: f32,
    gamma: f32,
) -> (u32, u32, f32) {
    let mut game = Game::new();
    let mut total_reward = 0.0;
    let mut turns = 0;

    let mut rng = rand::thread_rng();

    while !game.game_over && turns < 5000 { // 無限ループ防止のため最大5000ターンで打ち切り
        turns += 1;

        // 候補手を列挙
        let candidates = enumerate_all_moves(&game, model, None, 0);
        if candidates.is_empty() {
            // 置ける場所がない場合はゲームオーバーにする
            game.game_over = true;
            break;
        }

        // 行動の選択（epsilon-greedy）
        let chosen_move: CandidateMove = if rng.r#gen::<f32>() < epsilon {
            // ランダムに探索
            let idx = rng.gen_range(0..candidates.len());
            candidates[idx].clone()
        } else {
            // 最善手（評価値最大。enumerate_all_movesはソート済みなので0番目）
            candidates[0].clone()
        };

        // 状態遷移のシミュレートと反映
        // ホールドが選択された場合、ゲーム側のホールドを実行
        let held_i_piece = chosen_move.use_hold
            && game.current_piece.block_type == BlockType::I
            && game.hold_piece != Some(BlockType::I);

        if chosen_move.use_hold {
            game.hold();
        }

        // 配置座標と回転を合わせる
        game.current_piece.x = chosen_move.final_piece.x;
        game.current_piece.rotation = chosen_move.final_piece.rotation;
        game.current_piece.y = chosen_move.final_piece.y;

        // 特徴量（更新前）のコピー
        let phi_s = chosen_move.features.clone();
        let v_s = chosen_move.eval_score;

        // 固定してライン消去（この時点でnext_pieceがスポーンし、game_over判定も走る）
        let prev_lines = game.lines_cleared;
        game.lock_piece();
        let lines_cleared_this_turn = game.lines_cleared - prev_lines;

        // 報酬の設計
        let mut reward = crate::config::rl::SURVIVAL_REWARD;

        // Iミノをホールドへ保管した場合にボーナス（4ライン消し準備の促進）
        if held_i_piece {
            reward += crate::config::rl::HOLD_I_BONUS;
        }

        if (lines_cleared_this_turn as usize) < crate::config::rl::LINE_CLEAR_REWARDS.len() {
            reward += crate::config::rl::LINE_CLEAR_REWARDS[lines_cleared_this_turn as usize];
        }

        // 縦3マス以上の深い穴が1列しかない場合の報酬を追加
        let well_bonus_score = get_well_bonus(&game.board);
        if well_bonus_score > 0 {
            let rl_bonus = (well_bonus_score as f32) * crate::config::rl::WELL_BONUS_MULTIPLIER;
            reward += rl_bonus;
        }

        if game.game_over {
            reward += crate::config::rl::GAME_OVER_PENALTY;
        }
        total_reward += reward;

        // 遷移先状態における最善手の評価値 V(s') の取得
        let v_s_prime = if game.game_over {
            0.0 // 終端状態の価値は0
        } else {
            let next_candidates = enumerate_all_moves(&game, model, None, 0);
            if next_candidates.is_empty() {
                0.0
            } else {
                next_candidates[0].eval_score // ソート済みの最大値
            }
        };

        // TD誤差の計算
        let td_target = reward + gamma * v_s_prime;
        let td_error = td_target - v_s;

        // 重みの更新: w <- w + alpha * td_error * phi(s)
        for i in 0..model.weights.len() {
            model.weights[i] += alpha * td_error * phi_s[i];
        }

        // 重みが極端に発散するのを防ぐため、シンプルなクリッピングを施す
        // 重みのスケールがおかしくならないように最大/最小値を制限（L2正規化の代わり）
        for w in &mut model.weights {
            *w = w.clamp(-20.0, 20.0);
        }
    }

    (game.lines_cleared, turns, total_reward)
}

// 多数のエピソードを実行して強化学習を進める
#[allow(dead_code)]
pub fn train_rl<F>(
    model: &mut AiModel,
    num_episodes: usize,
    alpha: f32,
    gamma: f32,
    mut epsilon: f32,
    epsilon_decay: f32,
    min_epsilon: f32,
    mut progress_callback: F,
) where
    F: FnMut(usize, u32, u32, f32, &[f32]),
{
    for ep in 1..=num_episodes {
        let (lines, turns, reward) = run_rl_episode(model, epsilon, alpha, gamma);
        
        // 進捗の報告
        progress_callback(ep, lines, turns, reward, &model.weights);

        // 探索率の減衰
        epsilon = (epsilon * epsilon_decay).max(min_epsilon);
    }
}

#[allow(dead_code)]
pub fn set_board_from_strings(board: &mut crate::tetris::Board, board_map: &[String]) {
    *board = [[None; crate::tetris::BOARD_WIDTH]; crate::tetris::INTERNAL_HEIGHT];
    let nrows = board_map.len();
    for (r, row_str) in board_map.iter().enumerate() {
        let board_y = crate::tetris::INTERNAL_HEIGHT - nrows + r;
        if board_y >= crate::tetris::INTERNAL_HEIGHT { continue; }
        for (c, ch) in row_str.chars().enumerate() {
            if c >= crate::tetris::BOARD_WIDTH { break; }
            let bt = match ch {
                'i' | '1' => Some(BlockType::I),
                'o' | '2' => Some(BlockType::O),
                't' | '3' => Some(BlockType::T),
                's' | '4' => Some(BlockType::S),
                'z' | '5' => Some(BlockType::Z),
                'j' | '6' => Some(BlockType::J),
                'l' | '7' => Some(BlockType::L),
                '・' => Some(BlockType::I), // Map wildcard to filled block for training initialization
                _ => None,
            };
            board[board_y][c] = bt;
        }
    }
}

#[allow(dead_code)]
pub fn run_rl_t_spin_training_episode(
    model: &mut AiModel,
    epsilon: f32,
    alpha: f32,
    gamma: f32,
    base_map: &[String],
    next_pieces_non_mirrored: &[BlockType],
) -> (bool, u32, f32) {
    let mut game = Game::new();
    let mut rng = rand::thread_rng();

    // ランダムで左右反転する (50%)
    let mirror: bool = rng.r#gen();

    let final_map = if mirror {
        base_map.iter().map(|row| {
            row.chars().rev().map(|ch| {
                match ch {
                    'j' => 'l',
                    'l' => 'j',
                    's' => 'z',
                    'z' => 's',
                    other => other,
                }
            }).collect::<String>()
        }).collect::<Vec<String>>()
    } else {
        base_map.to_vec()
    };

    set_board_from_strings(&mut game.board, &final_map);

    let mut next_pieces = next_pieces_non_mirrored.to_vec();
    if mirror {
        for p in next_pieces.iter_mut() {
            *p = match *p {
                BlockType::J => BlockType::L,
                BlockType::L => BlockType::J,
                BlockType::S => BlockType::Z,
                BlockType::Z => BlockType::S,
                other => other,
            };
        }
    }

    if next_pieces.is_empty() {
        next_pieces.push(BlockType::T);
    }

    // ミノ順を固定
    game.current_piece = Piece::new(next_pieces[0]);
    // 既存のキューの先頭に、指定された残りのミノを挿入
    for (i, &p) in next_pieces.iter().skip(1).enumerate() {
        game.bag.queue.insert(i, p);
    }
    
    let mut total_reward = 0.0;
    let mut turns = 0;
    let mut locked_count = 0;
    let mut t_spin_detected = false;
    
    let mut prev_t_slots = crate::tetris::count_t_slots(&game.board);

    while !game.game_over && locked_count < 2 {
        turns += 1;

        let candidates = enumerate_all_moves(&game, model, None, 0);
        if candidates.is_empty() {
            game.game_over = true;
            break;
        }

        let chosen_move: CandidateMove = if rng.r#gen::<f32>() < epsilon {
            let idx = rng.gen_range(0..candidates.len());
            candidates[idx].clone()
        } else {
            candidates[0].clone()
        };

        if chosen_move.use_hold {
            game.hold();
        }

        game.current_piece.x = chosen_move.final_piece.x;
        game.current_piece.rotation = chosen_move.final_piece.rotation;
        game.current_piece.y = chosen_move.final_piece.y;

        let phi_s = chosen_move.features.clone();
        let v_s = chosen_move.eval_score;

        let prev_lines = game.lines_cleared;
        game.lock_piece();
        locked_count += 1;
        let lines_cleared_this_turn = game.lines_cleared - prev_lines;

        // Reward calculation
        let mut reward = 0.0;

        // T-spin check
        if let Some(ref spin_name) = game.last_t_spin {
            if spin_name.contains("Double") {
                reward += 1000.0;
                t_spin_detected = true;
            } else if spin_name.contains("Single") {
                reward += 500.0;
                t_spin_detected = true;
            } else {
                reward += 200.0;
                t_spin_detected = true;
            }
        }

        // T-slots change reward
        let curr_t_slots = crate::tetris::count_t_slots(&game.board);
        if curr_t_slots > prev_t_slots {
            reward += 150.0;
        } else if curr_t_slots < prev_t_slots && !t_spin_detected {
            reward -= 100.0;
        }
        prev_t_slots = curr_t_slots;

        if lines_cleared_this_turn > 0 && game.last_t_spin.is_none() {
            reward -= 100.0;
        }

        if game.game_over {
            reward -= 200.0;
        }

        total_reward += reward;

        let v_s_prime = if game.game_over || t_spin_detected {
            0.0
        } else {
            let next_candidates = enumerate_all_moves(&game, model, None, 0);
            if next_candidates.is_empty() {
                0.0
            } else {
                next_candidates[0].eval_score
            }
        };

        let td_target = reward + gamma * v_s_prime;
        let td_error = td_target - v_s;

        for i in 0..model.weights.len() {
            model.weights[i] += alpha * td_error * phi_s[i];
        }

        for w in &mut model.weights {
            *w = w.clamp(-50.0, 150.0);
        }

        if t_spin_detected {
            break;
        }
    }

    (t_spin_detected, turns, total_reward)
}
