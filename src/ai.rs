use crate::tetris::{Game, Piece, Board, BlockType, BOARD_WIDTH, INTERNAL_HEIGHT, get_well_bonus};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GpuBackendSelection {
    Auto,
    Rocm,
    Vulkan,
    Cpu,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AiModel {
    pub weights: Vec<f32>, // 特徴量に対応する重み
    #[serde(default)]
    pub is_nonlinear: bool,
    #[serde(default)]
    pub backend: Option<GpuBackendSelection>,
}

impl AiModel {
    pub fn new_default() -> Self {
        AiModel {
            weights: crate::config::heuristic::DEFAULT_WEIGHTS.to_vec(),
            is_nonlinear: false,
            backend: Some(GpuBackendSelection::Auto),
        }
    }

    /// addplan.md に準拠した20特徴量のハイブリッド非線形評価モデル（1000回最適化チューニング済み）
    pub fn new_20_feature_default() -> Self {
        AiModel {
            weights: vec![
                88.07,   // x0: TSpin (Single/Double/Triple/Mini) - 強力に強化
                61.85,   // x1: TSpinTerrain (TSlot, Corner, Rotation, Overhang) - 構築強化
                -25.93,  // x2: HolePenalty (Holes, Depth, Buried)
                -12.13,  // x3: HoleSpreadPenalty (Variance, Manhattan)
                22.55,   // x4: PlacementQuality (Mobility, Landing Quality)
                89.02,   // x5: Tetris (4-line clears)
                -19.19,  // x6: PureSinglePenalty (Single without REN/T-spin) - 無駄消し抑制
                -17.10,  // x7: PureDoublePenalty (Double without REN/T-spin)
                -8.01,   // x8: PureTriplePenalty (Triple without REN/T-spin)
                18.35,   // x9: REN (Combo chaining)
                31.47,   // x10: BTB (Back-to-Back status)
                11.64,   // x11: MaxCombo
                16.15,   // x12: MeanCombo
                96.14,   // x13: Perfect Clear (PC)
                -20.54,  // x14: HeightPenalty (Aggregate height)
                -23.53,  // x15: MaxHeightPenalty (Max column height)
                -6.64,   // x16: BumpinessPenalty (Height differences)
                25.08,   // x17: WellQuality (Gaussian well depth bonus)
                -25.35,  // x18: OverhangPenalty (Floating blocks)
                32.86,   // x19: FutureFit (Next queue & Hold piece compatibility)
            ],
            is_nonlinear: true,
            backend: Some(GpuBackendSelection::Auto),
        }
    }

    // 評価値を計算（高いほど良い）
    #[allow(dead_code)]
    pub fn evaluate(&self, features: &[f32]) -> f32 {
        let mut score = 0.0;
        for i in 0..self.weights.len().min(features.len()) {
            score += self.weights[i] * features[i];
        }
        score
    }

    // GPU Compute Shaderを用いた一括評価処理
    pub fn evaluate_batch(&self, feature_batch: &[Vec<f32>]) -> Vec<f32> {
        self.evaluate_batch_with_backend(feature_batch, self.backend.unwrap_or(GpuBackendSelection::Auto))
    }

    pub fn evaluate_batch_with_backend(&self, feature_batch: &[Vec<f32>], backend: GpuBackendSelection) -> Vec<f32> {
        if feature_batch.is_empty() {
            return Vec::new();
        }

        match backend {
            GpuBackendSelection::Rocm => {
                if let Some(scores) = crate::hip::get_hip_evaluator().evaluate_batch(&self.weights, feature_batch, self.is_nonlinear) {
                    return scores;
                }
                crate::gpu::get_gpu_evaluator().evaluate_batch(&self.weights, feature_batch, self.is_nonlinear)
            }
            GpuBackendSelection::Vulkan => {
                crate::gpu::get_gpu_evaluator().evaluate_batch(&self.weights, feature_batch, self.is_nonlinear)
            }
            GpuBackendSelection::Cpu => {
                let num_features = self.weights.len();
                feature_batch.iter().map(|feats| {
                    let mut s = 0.0f32;
                    for i in 0..num_features.min(feats.len()) {
                        s += self.weights[i] * feats[i];
                    }
                    s
                }).collect()
            }
            GpuBackendSelection::Auto => {
                if let Some(scores) = crate::hip::get_hip_evaluator().evaluate_batch(&self.weights, feature_batch, self.is_nonlinear) {
                    return scores;
                }
                crate::gpu::get_gpu_evaluator().evaluate_batch(&self.weights, feature_batch, self.is_nonlinear)
            }
        }
    }
}

#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct CandidateMove {
    pub x: i32,
    pub rotation: usize,
    pub use_hold: bool,
    pub features: Vec<f32>,
    pub eval_score: f32,
    pub final_piece: Piece,
}

// 盤面の特徴量を抽出する（9項目ベースライン）
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

    let raw_max_height = *heights.iter().max().unwrap_or(&0) as f32;
    let max_height = if raw_max_height <= 8.0 { 0.0 } else { raw_max_height };

    let avg_height = if raw_max_height <= 8.0 {
        0.0
    } else {
        (heights.iter().sum::<i32>() as f32) / (BOARD_WIDTH as f32)
    };

    let mut bumpiness = 0;
    for x in 0..(BOARD_WIDTH - 1) {
        bumpiness += (heights[x] - heights[x + 1]).abs();
    }
    let bumpiness = bumpiness as f32;

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
                holes += 1;
                blocks_above_holes += block_count_above_hole;
            }
        }
    }

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
    let t_slots = crate::tetris::count_t_slots(board) as f32;

    vec![
        max_height,
        avg_height,
        bumpiness,
        holes as f32,
        blocks_above_holes as f32,
        wells_depth as f32,
        cleared_1_3,
        cleared_4,
        t_slots,
    ]
}

/// addplan.md Section 2〜4 に準拠した 20次元正規化特徴量抽出
pub fn extract_20_features(
    game: &Game,
    board_after_clear: &Board,
    cleared_lines: usize,
    placed_piece: &Piece,
    use_hold: bool,
) -> Vec<f32> {
    let mut heights = [0; BOARD_WIDTH];
    for x in 0..BOARD_WIDTH {
        for y in 0..INTERNAL_HEIGHT {
            if board_after_clear[y][x].is_some() {
                heights[x] = (INTERNAL_HEIGHT - y) as i32;
                break;
            }
        }
    }

    let max_h = *heights.iter().max().unwrap_or(&0) as f32;
    let sum_h: i32 = heights.iter().sum();

    // 1. TSpin (0.0..1.2) - 候補手の配置でT-spin条件を満たすか直接判定
    let is_t_spin = if placed_piece.block_type == BlockType::T {
        let cx = placed_piece.x;
        let cy = placed_piece.y;
        let corners = [
            (cx - 1, cy - 1),
            (cx + 1, cy - 1),
            (cx - 1, cy + 1),
            (cx + 1, cy + 1),
        ];
        let mut filled_corners = 0;
        for &(x, y) in &corners {
            if x < 0 || x >= BOARD_WIDTH as i32 || y < 0 || y >= INTERNAL_HEIGHT as i32 {
                filled_corners += 1;
            } else if game.board[y as usize][x as usize].is_some() {
                filled_corners += 1;
            }
        }
        filled_corners >= 3
    } else {
        false
    };

    let t_spin_score = if is_t_spin {
        match cleared_lines {
            0 => 0.4,
            1 => 0.8, // T-Spin Single (TSS)
            2 => 1.0, // T-Spin Double (TSD)
            3 => 1.2, // T-Spin Triple (TST)
            _ => 0.5,
        }
    } else {
        0.0
    };

    // 2. TSpinTerrain (T-slots, corner support, depth, overhang)
    let t_slot_count = crate::tetris::count_t_slots(board_after_clear) as f32;
    let mut t_spin_terrain = crate::tetris::evaluate_t_spin_terrain(board_after_clear);
    let next_has_t = game.hold_piece == Some(BlockType::T) || game.bag.peek_next(4).contains(&BlockType::T);
    if next_has_t && t_spin_terrain > 0.3 {
        t_spin_terrain = (t_spin_terrain + 0.3).min(1.0);
    }

    // 3. Holes & Hole Depth & Buried Holes
    let mut holes = 0;
    let mut col_holes = [0; BOARD_WIDTH];
    let mut hole_coords: Vec<(usize, usize)> = Vec::new();
    let mut blocks_above = 0;

    for x in 0..BOARD_WIDTH {
        let mut block_found = false;
        let mut count_above = 0;
        for y in 0..INTERNAL_HEIGHT {
            if board_after_clear[y][x].is_some() {
                block_found = true;
                count_above += 1;
            } else if block_found {
                holes += 1;
                col_holes[x] += 1;
                hole_coords.push((x, y));
                blocks_above += count_above;
            }
        }
    }
    let hole_penalty = ((holes as f32 * 1.0 + blocks_above as f32 * 0.5) / 10.0).min(1.0);

    // 4. Hole Spread Penalty (列分散 + 穴間マンハッタン距離)
    let mean_h_per_col = holes as f32 / BOARD_WIDTH as f32;
    let variance = col_holes.iter().map(|&h| (h as f32 - mean_h_per_col).powi(2)).sum::<f32>() / BOARD_WIDTH as f32;
    let mut spread_dist = 0.0f32;
    let mut pairs = 0;
    for i in 0..hole_coords.len() {
        for j in (i + 1)..hole_coords.len() {
            let dx = (hole_coords[i].0 as i32 - hole_coords[j].0 as i32).abs() as f32;
            let dy = (hole_coords[i].1 as i32 - hole_coords[j].1 as i32).abs() as f32;
            spread_dist += dx + dy;
            pairs += 1;
        }
    }
    let mean_spread = if pairs > 0 { spread_dist / pairs as f32 } else { 0.0 };
    let hole_spread_penalty = ((variance * 0.5 + mean_spread * 0.5) / 10.0).min(1.0);

    // 5. Placement Quality (着地位置の適合度)
    let placement_quality = if placed_piece.y >= (INTERNAL_HEIGHT as i32 - 6) { 0.9 } else { 0.5 };

    // 6. Tetris (4-line clear)
    let tetris_score = if cleared_lines == 4 { 1.0 } else { 0.0 };

    // 7, 8, 9. Pure Single, Double, Triple
    let is_tspin_or_ren = game.last_t_spin.is_some() || game.btb;
    let pure_single = if cleared_lines == 1 && !is_tspin_or_ren { 1.0 } else { 0.0 };
    let pure_double = if cleared_lines == 2 && !is_tspin_or_ren { 1.0 } else { 0.0 };
    let pure_triple = if cleared_lines == 3 && !is_tspin_or_ren { 1.0 } else { 0.0 };

    // 10. REN (Combo)
    let ren_score = 0.0f32;
    // 11. BTB
    let btb_score = if game.btb { 1.0 } else { 0.0 };
    // 12. MaxCombo, 13. MeanCombo
    let max_combo = 0.0f32;
    let mean_combo = 0.0f32;

    // 14. Perfect Clear
    let is_pc = board_after_clear.iter().all(|row| row.iter().all(|c| c.is_none()));
    let pc_score = if is_pc { 1.0 } else { 0.0 };

    // 15. Height Penalty (Aggregate height)
    let height_penalty = (sum_h as f32 / (BOARD_WIDTH * 15) as f32).min(1.0);

    // 16. Max Height Penalty
    let max_height_penalty = (max_h / 20.0).min(1.0);

    // 17. Bumpiness Penalty
    let mut bumpiness = 0;
    for x in 0..(BOARD_WIDTH - 1) {
        bumpiness += (heights[x] - heights[x + 1]).abs();
    }
    let bumpiness_penalty = (bumpiness as f32 / 30.0).min(1.0);

    // 18. Well Quality (Gaussian around optimal depth 4 on column 0 or 9)
    let well_col_0 = if heights[1] > heights[0] { heights[1] - heights[0] } else { 0 };
    let well_col_9 = if heights[8] > heights[9] { heights[8] - heights[9] } else { 0 };
    let max_well_depth = well_col_0.max(well_col_9) as f32;
    let well_quality = (-((max_well_depth - 4.0).powi(2)) / 8.0).exp();

    // 19. Overhang Penalty (Discount overhangs that belong to valid T-slots)
    let mut overhangs = 0;
    for x in 0..BOARD_WIDTH {
        for y in 0..(INTERNAL_HEIGHT - 1) {
            if board_after_clear[y][x].is_some() && board_after_clear[y + 1][x].is_none() {
                overhangs += 1;
            }
        }
    }
    let effective_overhangs = (overhangs as f32 - (t_slot_count * 1.5)).max(0.0);
    let overhang_penalty = (effective_overhangs / 8.0).min(1.0);

    // 20. Future Fit (Next queue / Hold fit, Hoiko-style HoldT synergy & WasteT penalty)
    let mut future_fit = if use_hold { 0.8f32 } else { 0.7f32 };
    if next_has_t && t_spin_terrain > 0.4 {
        future_fit = 1.0f32;
    }
    // HoldT: Tミノをホールド温存している状態でTスロット構築中の場合はボーナス
    if game.hold_piece == Some(BlockType::T) && t_spin_terrain > 0.3 {
        future_fit = (future_fit + 0.2f32).min(1.0f32);
    }
    // WasteT: 盤面にTスロットがあるのにTミノを通常平積みに無駄消費した場合は大幅減点
    if placed_piece.block_type == BlockType::T && t_slot_count > 0.0 && t_spin_score == 0.0 {
        future_fit = (future_fit - 0.5f32).max(0.0f32);
    }

    vec![
        t_spin_score,
        t_spin_terrain,
        hole_penalty,
        hole_spread_penalty,
        placement_quality,
        tetris_score,
        pure_single,
        pure_double,
        pure_triple,
        ren_score,
        btb_score,
        max_combo,
        mean_combo,
        pc_score,
        height_penalty,
        max_height_penalty,
        bumpiness_penalty,
        well_quality,
        overhang_penalty,
        future_fit,
    ]
}

// すべての可能な配置（候補手）を列挙する（先読みなしのベース版）
pub fn enumerate_all_moves_base(
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
                game.bag.peek_next(1)[0]
            }
        };
        enumerate_moves_for_piece(game, next_piece_type, true, model, opening, opening_turn, &mut moves);
    }

    // 評価スコアの高い順にソート
    moves.sort_by(|a, b| b.eval_score.partial_cmp(&a.eval_score).unwrap_or(std::cmp::Ordering::Equal));
    moves
}

// すべての可能な配置（候補手）を列挙し、Nextキューに基づき将来の盤面評価を先読み(Lookahead / Beam Search)してGPUバッチでスコアを再計算する
pub fn enumerate_all_moves(
    game: &Game,
    model: &AiModel,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
) -> Vec<CandidateMove> {
    let num_nexts = game.bag.peek_next(5).len();
    beam_search(game, model, num_nexts, 50, opening, opening_turn)
}

/// GPUアクセラレーション対応 Beam Search (ビーム探索) 先読みエンジン
pub fn beam_search(
    game: &Game,
    model: &AiModel,
    depth: usize,
    beam_width: usize,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
) -> Vec<CandidateMove> {
    let mut root_moves = enumerate_all_moves_base(game, model, opening, opening_turn);

    let is_opening_active = opening
        .map_or(false, |o| game.lines_cleared < o.active_until_lines);
    if is_opening_active || root_moves.is_empty() || depth == 0 {
        return root_moves;
    }

    let actual_beam_width = root_moves.len().min(beam_width);
    let discount = crate::config::heuristic::LOOKAHEAD_DISCOUNT_FACTOR;

    struct BranchState {
        move_idx: usize,
        game: Game,
        accumulated_score: f32,
        current_discount: f32,
        is_game_over: bool,
    }

    let mut branches: Vec<BranchState> = root_moves
        .iter()
        .take(actual_beam_width)
        .enumerate()
        .map(|(idx, m)| {
            let mut temp_game = game.clone();
            if m.use_hold {
                temp_game.hold();
            }
            temp_game.current_piece.x = m.final_piece.x;
            temp_game.current_piece.rotation = m.final_piece.rotation;
            temp_game.hard_drop();

            BranchState {
                move_idx: idx,
                game: temp_game,
                accumulated_score: m.eval_score,
                current_discount: discount,
                is_game_over: false,
            }
        })
        .collect();

    for turn_offset in 0..depth {
        let curr_turn = opening_turn + 1 + turn_offset;

        for branch in branches.iter_mut() {
            if branch.is_game_over {
                continue;
            }
            if branch.game.game_over {
                branch.accumulated_score += crate::config::rl::GAME_OVER_PENALTY * branch.current_discount;
                branch.is_game_over = true;
                continue;
            }

            let branch_moves = enumerate_all_moves_base(&branch.game, model, opening, curr_turn);
            if branch_moves.is_empty() {
                branch.accumulated_score += crate::config::rl::GAME_OVER_PENALTY * branch.current_discount;
                branch.is_game_over = true;
                continue;
            }

            let best_next = &branch_moves[0];
            branch.accumulated_score += best_next.eval_score * branch.current_discount;
            branch.current_discount *= discount;

            if best_next.use_hold {
                branch.game.hold();
            }
            branch.game.current_piece.x = best_next.final_piece.x;
            branch.game.current_piece.rotation = best_next.final_piece.rotation;
            branch.game.hard_drop();

            if branch.game.game_over {
                branch.accumulated_score += crate::config::rl::GAME_OVER_PENALTY * branch.current_discount;
                branch.is_game_over = true;
            }
        }
    }

    for branch in branches {
        root_moves[branch.move_idx].eval_score = branch.accumulated_score;
    }

    root_moves.sort_by(|a, b| b.eval_score.partial_cmp(&a.eval_score).unwrap_or(std::cmp::Ordering::Equal));
    root_moves
}

// BFSによる全到達可能着地位置の探索 (Reachability Search Engine)
pub fn search_reachable_landings(game: &Game, block_type: BlockType) -> Vec<Piece> {
    let mut landings = Vec::new();
    let mut visited = [[[false; 4]; 16]; INTERNAL_HEIGHT];
    let mut queue = VecDeque::new();

    let spawn_piece = Piece::new(block_type);

    if !game.is_valid_position(&spawn_piece) {
        return landings;
    }

    let start_x_idx = (spawn_piece.x + 3) as usize;
    if start_x_idx < 16 && (spawn_piece.y as usize) < INTERNAL_HEIGHT {
        visited[spawn_piece.y as usize][start_x_idx][spawn_piece.rotation] = true;
        queue.push_back(spawn_piece);
    }

    let mut landing_visited = [[[false; 4]; 16]; INTERNAL_HEIGHT];

    while let Some(curr) = queue.pop_front() {
        let down_piece = Piece {
            block_type: curr.block_type,
            x: curr.x,
            y: curr.y + 1,
            rotation: curr.rotation,
        };
        let is_landing = !game.is_valid_position(&down_piece);

        if is_landing {
            let cx_idx = (curr.x + 3) as usize;
            let cy_idx = curr.y as usize;
            if cy_idx < INTERNAL_HEIGHT && cx_idx < 16 {
                if !landing_visited[cy_idx][cx_idx][curr.rotation] {
                    landing_visited[cy_idx][cx_idx][curr.rotation] = true;
                    landings.push(curr.clone());
                }
            }
        }

        // 1. 左移動
        let left_piece = Piece { x: curr.x - 1, y: curr.y, rotation: curr.rotation, block_type: curr.block_type };
        try_enqueue_reachable(game, left_piece, &mut visited, &mut queue);

        // 2. 右移動
        let right_piece = Piece { x: curr.x + 1, y: curr.y, rotation: curr.rotation, block_type: curr.block_type };
        try_enqueue_reachable(game, right_piece, &mut visited, &mut queue);

        // 3. ソフトドロップ (下移動)
        if !is_landing {
            try_enqueue_reachable(game, down_piece, &mut visited, &mut queue);
        }

        // Oミノ以外は回転移動（SRSウォールキック含む）を探索
        if block_type != BlockType::O {
            let from_rot = curr.rotation;

            // 4. 時計回り (CW)
            let to_rot_cw = (from_rot + 1) % 4;
            let kick_offsets_cw = game.get_kick_offsets(block_type, from_rot, to_rot_cw);
            for &(dx, dy) in &kick_offsets_cw {
                let test_piece = Piece {
                    block_type,
                    x: curr.x + dx,
                    y: curr.y + dy,
                    rotation: to_rot_cw,
                };
                if game.is_valid_position(&test_piece) {
                    try_enqueue_reachable(game, test_piece, &mut visited, &mut queue);
                    break;
                }
            }

            // 5. 反時計回り (CCW)
            let to_rot_ccw = (from_rot + 3) % 4;
            let kick_offsets_ccw = game.get_kick_offsets(block_type, from_rot, to_rot_ccw);
            for &(dx, dy) in &kick_offsets_ccw {
                let test_piece = Piece {
                    block_type,
                    x: curr.x + dx,
                    y: curr.y + dy,
                    rotation: to_rot_ccw,
                };
                if game.is_valid_position(&test_piece) {
                    try_enqueue_reachable(game, test_piece, &mut visited, &mut queue);
                    break;
                }
            }
        }
    }

    landings
}

fn try_enqueue_reachable(
    game: &Game,
    piece: Piece,
    visited: &mut [[[bool; 4]; 16]; INTERNAL_HEIGHT],
    queue: &mut VecDeque<Piece>,
) {
    if game.is_valid_position(&piece) {
        let x_idx = (piece.x + 3) as usize;
        let y_idx = piece.y as usize;
        let rot_idx = piece.rotation;
        if y_idx < INTERNAL_HEIGHT && x_idx < 16 {
            if !visited[y_idx][x_idx][rot_idx] {
                visited[y_idx][x_idx][rot_idx] = true;
                queue.push_back(piece);
            }
        }
    }
}

// 特定のミノ種について、到達可能な配置候補をBFSで全探索して moves に追加
fn enumerate_moves_for_piece(
    game: &Game,
    block_type: BlockType,
    use_hold: bool,
    model: &AiModel,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
    moves: &mut Vec<CandidateMove>,
) {
    let landings = search_reachable_landings(game, block_type);

    struct TempCandidate {
        target_piece: Piece,
        temp_board_after_clear: Board,
        features: Vec<f32>,
    }

    let mut temp_candidates = Vec::new();
    let use_20_features = model.weights.len() == 20;

    for target_piece in landings {
        let mut temp_board = game.board;
        let mut cells_locked_count = 0;
        for &(cx, cy) in &target_piece.get_cells() {
            if cx >= 0 && cx < BOARD_WIDTH as i32 && cy >= 0 && cy < INTERNAL_HEIGHT as i32 {
                temp_board[cy as usize][cx as usize] = Some(block_type);
                cells_locked_count += 1;
            }
        }

        if cells_locked_count == 4 {
            let (temp_board_after_clear, cleared) = simulate_line_clears(&temp_board);
            let features = if use_20_features {
                extract_20_features(game, &temp_board_after_clear, cleared, &target_piece, use_hold)
            } else {
                extract_features(&temp_board_after_clear, cleared)
            };

            temp_candidates.push(TempCandidate {
                target_piece,
                temp_board_after_clear,
                features,
            });
        }
    }

    if temp_candidates.is_empty() {
        return;
    }

    let feature_batch: Vec<Vec<f32>> = temp_candidates.iter().map(|c| c.features.clone()).collect();
    let gpu_scores = model.evaluate_batch(&feature_batch);

    let is_opening_active = opening
        .map_or(false, |o| game.lines_cleared < o.active_until_lines);

    for (i, c) in temp_candidates.into_iter().enumerate() {
        let target_x = c.target_piece.x;
        let rotation = c.target_piece.rotation;

        let mut eval_score = if is_opening_active {
            let o = opening.unwrap();
            let mut s = 0.0f32;
            for j in 0..o.opening_weights.len().min(c.features.len()) {
                s += o.opening_weights[j] * c.features[j];
            }
            s
        } else {
            gpu_scores[i]
        };

        if is_opening_active {
            eval_score += crate::opening::evaluate_opening_fit(
                &c.temp_board_after_clear,
                opening.unwrap(),
                game,
                opening_turn,
            );
            eval_score += crate::opening::evaluate_sequence_match(
                opening.unwrap(),
                game,
                opening_turn,
                block_type,
                target_x,
                rotation,
            );
        }

        if !is_opening_active && !use_20_features {
            // 縦3マス以上の深い穴ボーナス
            let well_bonus_score = get_well_bonus(&c.temp_board_after_clear);
            if well_bonus_score > 0 {
                let ai_bonus = (well_bonus_score as f32) * crate::config::heuristic::WELL_BONUS_MULTIPLIER;
                eval_score += ai_bonus;
            }

            // 4〜7列目のターゲット穴ペナルティ
            let mut has_target_hole = false;
            for x in 3..=6 {
                let mut block_found = false;
                for y in 0..INTERNAL_HEIGHT {
                    if c.temp_board_after_clear[y][x].is_some() {
                        block_found = true;
                    } else if block_found {
                        has_target_hole = true;
                        break;
                    }
                }
                if has_target_hole {
                    break;
                }
            }
            if has_target_hole {
                eval_score += crate::config::heuristic::TARGET_HOLE_PENALTY;
            }

            // 複数谷ペナルティ
            let mut heights = [0; BOARD_WIDTH];
            for col in 0..BOARD_WIDTH {
                let mut height = 0;
                for y in 0..INTERNAL_HEIGHT {
                    if c.temp_board_after_clear[y][col].is_some() {
                        height = INTERNAL_HEIGHT - y;
                        break;
                    }
                }
                heights[col] = height as i32;
            }

            let mut well_count = 0;
            for col in 0..BOARD_WIDTH {
                let left = if col == 0 { INTERNAL_HEIGHT as i32 } else { heights[col - 1] };
                let right = if col == BOARD_WIDTH - 1 { INTERNAL_HEIGHT as i32 } else { heights[col + 1] };
                let h = heights[col];
                let diff = std::cmp::min(left, right) - h;
                if diff >= 3 {
                    well_count += 1;
                }
            }

            if well_count >= 2 {
                eval_score += crate::config::heuristic::MULTIPLE_WELLS_PENALTY;
            }

            // 放置された穴ペナルティ
            let mut isolated_holes = 0;
            for x in 0..BOARD_WIDTH {
                for y in 1..INTERNAL_HEIGHT {
                    let is_empty = c.temp_board_after_clear[y][x].is_none();
                    let has_top = c.temp_board_after_clear[y - 1][x].is_some();
                    let has_bottom = y == INTERNAL_HEIGHT - 1 || c.temp_board_after_clear[y + 1][x].is_some();
                    if is_empty && has_top && has_bottom {
                        isolated_holes += 1;
                    }
                }
            }
            if isolated_holes > 0 {
                eval_score += (isolated_holes as f32) * crate::config::heuristic::ABANDONED_HOLE_PENALTY;
            }
        }

        // Iミノホールドボーナス
        if use_hold && game.current_piece.block_type == BlockType::I && game.hold_piece != Some(BlockType::I) {
            eval_score += crate::config::heuristic::HOLD_I_BONUS;
        }

        moves.push(CandidateMove {
            x: target_x,
            rotation,
            use_hold,
            features: c.features,
            eval_score,
            final_piece: c.target_piece,
        });
    }
}

pub fn simulate_future_moves(
    game: &Game,
    model: &AiModel,
    opening: Option<&crate::opening::OpeningTemplate>,
    opening_turn: usize,
) -> Vec<(Piece, BlockType)> {
    let mut future_pieces = Vec::new();
    let mut sim_game = game.clone();
    let next_pieces = sim_game.bag.peek_next(5);

    let is_opening_active = opening
        .map_or(false, |o| sim_game.lines_cleared < o.active_until_lines);

    let max_steps = if is_opening_active {
        if let Some(branch) = opening.unwrap().get_active_branch(&sim_game) {
            branch.parsed_placements.len().saturating_sub(opening_turn)
        } else {
            opening.unwrap().parsed_placements.len().saturating_sub(opening_turn)
        }
    } else {
        next_pieces.len().min(5)
    };

    let mut current_turn = opening_turn;

    for _ in 0..max_steps {
        if sim_game.game_over {
            break;
        }

        let candidates = enumerate_all_moves_base(&sim_game, model, opening, current_turn);
        if candidates.is_empty() {
            break;
        }

        let best_move = &candidates[0];
        let bt = best_move.final_piece.block_type;
        future_pieces.push((best_move.final_piece.clone(), bt));

        if best_move.use_hold {
            sim_game.hold();
        }
        sim_game.current_piece.x = best_move.final_piece.x;
        sim_game.current_piece.rotation = best_move.final_piece.rotation;
        sim_game.hard_drop();

        current_turn += 1;
    }

    future_pieces
}

fn simulate_line_clears(board: &Board) -> (Board, usize) {
    let mut new_board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
    let mut new_y = INTERNAL_HEIGHT - 1;
    let mut lines_cleared = 0;

    for y in (0..INTERNAL_HEIGHT).rev() {
        let is_full = board[y].iter().all(|cell| cell.is_some());
        if !is_full {
            new_board[new_y] = board[y];
            if new_y > 0 {
                new_y -= 1;
            }
        } else {
            lines_cleared += 1;
        }
    }

    (new_board, lines_cleared)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookahead_next_influence() {
        let game = Game::new();
        let model = AiModel::new_default();
        let moves = beam_search(&game, &model, 3, 30, None, 0);
        assert!(!moves.is_empty());
    }

    #[test]
    fn test_reachability_bfs_tspin_slot() {
        let mut game = Game::new();
        for x in 0..BOARD_WIDTH {
            if x != 4 && x != 5 && x != 6 {
                game.board[23][x] = Some(BlockType::I);
                game.board[22][x] = Some(BlockType::I);
            }
        }
        game.board[23][4] = Some(BlockType::I);
        game.board[23][6] = Some(BlockType::I);
        game.board[21][4] = Some(BlockType::I);

        let landings = search_reachable_landings(&game, BlockType::T);
        let found_tspin_landing = landings.iter().any(|p| p.x == 4 && p.rotation == 2);
        assert!(found_tspin_landing, "Reachability BFS should find the rotated T-piece landing inside the T-slot!");
    }

    #[test]
    fn test_gpu_beam_search_lookahead() {
        let game = Game::new();
        let model = AiModel::new_default();
        let moves = beam_search(&game, &model, 2, 20, None, 0);
        assert!(!moves.is_empty());
        for i in 0..(moves.len() - 1) {
            assert!(moves[i].eval_score >= moves[i+1].eval_score);
        }
    }

    #[test]
    fn test_20_features_extraction() {
        let game = Game::new();
        let piece = Piece::new(BlockType::T);
        let feats = extract_20_features(&game, &game.board, 0, &piece, false);
        assert_eq!(feats.len(), 20);
    }
}
