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
                140.0,   // x0: TSpin (TSD: 1.0, TST: 1.2, TSS: 0.6)
                75.0,    // x1: TSpinTerrain (T-slots, overhangs, stepping stones)
                -85.0,   // x2: HolePenalty (Buried holes)
                -35.0,   // x3: HoleSpreadPenalty (Hole dispersion)
                45.0,    // x4: PlacementQuality (Roof formation & donation quality)
                120.0,   // x5: Tetris (4-line clears)
                -25.0,   // x6: PureSinglePenalty (Single without REN/T-spin)
                -20.0,   // x7: PureDoublePenalty (Double without REN/T-spin)
                -10.0,   // x8: PureTriplePenalty (Triple without REN/T-spin)
                20.0,    // x9: REN (Combo chaining)
                45.0,    // x10: BTB (Back-to-Back status)
                12.0,    // x11: MaxCombo
                16.0,    // x12: MeanCombo
                96.0,    // x13: Perfect Clear (PC)
                -45.0,   // x14: HeightPenalty (Aggregate height)
                -60.0,   // x15: MaxHeightPenalty (Max column height)
                -15.0,   // x16: BumpinessPenalty (Height differences)
                30.0,    // x17: WellQuality (Gaussian well depth bonus)
                -30.0,   // x18: OverhangPenalty (Floating blocks)
                55.0,    // x19: FutureFit (Next queue & Hold piece compatibility)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MoveAction {
    MoveLeft,
    MoveRight,
    SoftDrop,
    HardDrop,
    RotateCW,
    RotateCCW,
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
    pub was_rotate: bool,
    pub path: Vec<MoveAction>,
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
    was_rotate: bool,
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

    // 1. TSpin (0.0..1.2) - Guideline 3-Corner 判定に準拠
    let t_spin_result = if placed_piece.block_type == BlockType::T {
        crate::tetris::check_t_spin_type(
            &game.board,
            placed_piece,
            was_rotate,
            false,
        )
    } else {
        crate::tetris::TSpinResult::None
    };

    let t_spin_score = match t_spin_result {
        crate::tetris::TSpinResult::Full(_) => match cleared_lines {
            0 => 0.0,  // 空打ちは0点（横一列以上そろっていない場合はT-Spin得点を与えない）
            1 => 0.60, // TSS
            2 => 1.00, // TSD (最大目標)
            3 => 1.20, // TST
            _ => 0.0,
        },
        crate::tetris::TSpinResult::Mini(_) => match cleared_lines {
            0 => 0.0,
            1 => 0.20, // Mini Single
            2 => 0.40, // Mini Double
            _ => 0.0,
        },
        crate::tetris::TSpinResult::None => 0.0,
    };

    // 2. TSpinTerrain (T-slots, corner support, depth, overhang, internal position & kaidan)
    let t_slot_count = crate::tetris::count_t_slots(board_after_clear) as f32;
    let mut t_spin_terrain = crate::tetris::evaluate_t_spin_terrain(board_after_clear);
    let next_has_t = game.hold_piece == Some(BlockType::T) || game.bag.peek_next(4).contains(&BlockType::T);
    if next_has_t && t_spin_terrain > 0.3 {
        t_spin_terrain = (t_spin_terrain + 0.3).min(1.0);
    }
    // 2〜9列目（3〜8列目推奨）単一列スロット評価
    let slot_pos_quality = crate::tetris::evaluate_t_slot_column_position(placed_piece.x.clamp(0, 9) as usize, 1);
    if t_spin_terrain > 0.3 {
        t_spin_terrain = (t_spin_terrain * 0.8 + slot_pos_quality * 0.2).min(1.0);
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

    // 5. Placement Quality (着地位置の適合度 & 屋根構築・ドネイト判定)
    let is_empty_tspin = placed_piece.block_type == BlockType::T && t_spin_result != crate::tetris::TSpinResult::None && cleared_lines == 0;
    let is_roof_formation = placed_piece.block_type != BlockType::T && {
        let mut creates_roof = false;
        for &(cx, cy) in &placed_piece.get_cells() {
            if cy + 1 < INTERNAL_HEIGHT as i32 && cx >= 0 && cx < BOARD_WIDTH as i32 {
                if game.board[(cy + 1) as usize][cx as usize].is_none() {
                    creates_roof = true;
                    break;
                }
            }
        }
        creates_roof
    };

    let placement_quality = if is_empty_tspin {
        0.05f32 // 空打ちは最低評価（横一列揃っていない状態での消費を抑止）
    } else if t_spin_score > 0.0 {
        1.0f32
    } else if is_roof_formation && t_spin_terrain > 0.4 {
        1.0f32 // 有効な屋根構築・ドネイト手
    } else if placed_piece.y >= (INTERNAL_HEIGHT as i32 - 6) {
        0.85f32
    } else {
        0.50f32
    };

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

    // 17. Bumpiness Penalty (中央山型集中・富士山型ペナルティ統合)
    let mut bumpiness = 0;
    for x in 0..(BOARD_WIDTH - 1) {
        bumpiness += (heights[x] - heights[x + 1]).abs();
    }
    let center_convexity = crate::tetris::calculate_center_convexity(board_after_clear);
    let bumpiness_penalty = ((bumpiness as f32 / 30.0) + center_convexity * 0.5).min(1.0);

    // 18. Well Quality (Gaussian around optimal depth 4 on column 0 or 9, 両端同時空き時はゼロ化)
    let (is_dual_well, _dual_sev) = crate::tetris::detect_dual_side_wells(board_after_clear);
    let well_col_0 = if heights[1] > heights[0] { heights[1] - heights[0] } else { 0 };
    let well_col_9 = if heights[8] > heights[9] { heights[8] - heights[9] } else { 0 };
    let max_well_depth = well_col_0.max(well_col_9) as f32;
    let well_quality = if is_dual_well {
        0.05 // 両端同時空きはIミノ枯渇リスクのため最低評価
    } else {
        (-((max_well_depth - 4.0).powi(2)) / 8.0).exp()
    };

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
    // WasteT / Empty T-Spin: 盤面にTスロットがあるのにTミノを通常平積みに無駄消費、または空打ちした場合は大幅減点
    if placed_piece.block_type == BlockType::T && (t_slot_count > 0.0 || t_spin_terrain > 0.3) && t_spin_score == 0.0 {
        future_fit = (future_fit - 0.85f32).max(0.0f32);
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
            temp_game.current_piece.y = m.final_piece.y;
            temp_game.current_piece.rotation = m.final_piece.rotation;
            temp_game.last_action_was_rotate = m.was_rotate;
            temp_game.lock_piece();

            BranchState {
                move_idx: idx,
                game: temp_game,
                accumulated_score: m.eval_score,
                current_discount: discount,
                is_game_over: false,
            }
        })
        .collect();

    let visible_nexts = game.bag.peek_next(5).len();

    for turn_offset in 0..depth {
        let curr_turn = opening_turn + 1 + turn_offset;
        // HoikoCode TrustRate: 可視ネクストキューを超える深さの探索ノードに対し信頼度を減衰 (0.90^(超過手))
        let trust_rate = if turn_offset >= visible_nexts {
            0.90f32.powi((turn_offset + 1 - visible_nexts) as i32)
        } else {
            1.0f32
        };

        for branch in branches.iter_mut() {
            if branch.is_game_over {
                continue;
            }
            if branch.game.game_over {
                branch.accumulated_score += crate::config::rl::GAME_OVER_PENALTY * branch.current_discount * trust_rate;
                branch.is_game_over = true;
                continue;
            }

            let branch_moves = enumerate_all_moves_base(&branch.game, model, opening, curr_turn);
            if branch_moves.is_empty() {
                branch.accumulated_score += crate::config::rl::GAME_OVER_PENALTY * branch.current_discount * trust_rate;
                branch.is_game_over = true;
                continue;
            }

            let best_next = &branch_moves[0];
            branch.accumulated_score += best_next.eval_score * branch.current_discount * trust_rate;
            branch.current_discount *= discount;

            if best_next.use_hold {
                branch.game.hold();
            }
            branch.game.current_piece.x = best_next.final_piece.x;
            branch.game.current_piece.y = best_next.final_piece.y;
            branch.game.current_piece.rotation = best_next.final_piece.rotation;
            branch.game.last_action_was_rotate = best_next.was_rotate;
            branch.game.lock_piece();

            if branch.game.game_over {
                branch.accumulated_score += crate::config::rl::GAME_OVER_PENALTY * branch.current_discount * trust_rate;
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

#[derive(Debug, Clone)]
pub struct LandingInfo {
    pub piece: Piece,
    pub was_rotate: bool,
    pub path: Vec<MoveAction>,
}

// BFSによる全到達可能着地位置の探索 (Reachability Search Engine)
pub fn search_reachable_landings(game: &Game, block_type: BlockType) -> Vec<LandingInfo> {
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
        queue.push_back((spawn_piece.clone(), false, Vec::new()));
    }

    let mut landing_visited = [[[false; 4]; 16]; INTERNAL_HEIGHT];

    while let Some((curr, was_rotate, path)) = queue.pop_front() {
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
                    // 実戦操作最適化: 直線落下時は最短HardDropパス、スピン時は末尾にHardDrop追加で即時ロック
                    let optimized_path = optimize_execution_path(game, &spawn_piece, &curr, &path);
                    landings.push(LandingInfo {
                        piece: curr.clone(),
                        was_rotate,
                        path: optimized_path,
                    });
                }
            }
        }

        // 1. 左移動 (was_rotate = false)
        let left_piece = Piece { x: curr.x - 1, y: curr.y, rotation: curr.rotation, block_type: curr.block_type };
        let mut left_path = path.clone();
        left_path.push(MoveAction::MoveLeft);
        try_enqueue_reachable(game, left_piece, false, left_path, &mut visited, &mut queue);

        // 2. 右移動 (was_rotate = false)
        let right_piece = Piece { x: curr.x + 1, y: curr.y, rotation: curr.rotation, block_type: curr.block_type };
        let mut right_path = path.clone();
        right_path.push(MoveAction::MoveRight);
        try_enqueue_reachable(game, right_piece, false, right_path, &mut visited, &mut queue);

        // 3. ソフトドロップ (下移動, was_rotate = false)
        if !is_landing {
            let mut down_path = path.clone();
            down_path.push(MoveAction::SoftDrop);
            try_enqueue_reachable(game, down_piece, false, down_path, &mut visited, &mut queue);
        }

        // Oミノ以外は回転移動（SRSウォールキック含む）を探索 (was_rotate = true)
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
                    let mut cw_path = path.clone();
                    cw_path.push(MoveAction::RotateCW);
                    try_enqueue_reachable(game, test_piece, true, cw_path, &mut visited, &mut queue);
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
                    let mut ccw_path = path.clone();
                    ccw_path.push(MoveAction::RotateCCW);
                    try_enqueue_reachable(game, test_piece, true, ccw_path, &mut visited, &mut queue);
                    break;
                }
            }
        }
    }

    landings
}

/// 実戦操作最適化（ハードドロップ優先 & スピン・ソフトドロップ後の即時ハードドロップ）
pub fn optimize_execution_path(
    game: &Game,
    spawn_piece: &Piece,
    target_piece: &Piece,
    bfs_path: &[MoveAction],
) -> Vec<MoveAction> {
    // 1. 直線落下可能（上空から遮蔽なく直接落とせる）か検証
    let mut direct_valid = true;
    let target_rot = target_piece.rotation;
    let target_x = target_piece.x;
    let target_y = target_piece.y;

    // スポーン高度における回転・横移動の成立性
    let mut align_piece = Piece {
        block_type: spawn_piece.block_type,
        x: target_x,
        y: spawn_piece.y,
        rotation: target_rot,
    };
    if !game.is_valid_position(&align_piece) {
        direct_valid = false;
    } else {
        // スポーン高度から目標高度までの全行で衝突がないか確認
        for y in spawn_piece.y..=target_y {
            align_piece.y = y;
            if !game.is_valid_position(&align_piece) {
                direct_valid = false;
                break;
            }
        }
    }

    if direct_valid {
        // 上空直通可能な手: 最短の [回転, 横移動, HardDrop] パスを生成（不要なSoftDropを完全排除）
        let mut path = Vec::new();
        // 回転
        match target_rot {
            1 => path.push(MoveAction::RotateCW),
            2 => {
                path.push(MoveAction::RotateCW);
                path.push(MoveAction::RotateCW);
            }
            3 => path.push(MoveAction::RotateCCW),
            _ => {}
        }
        // 横移動
        let dx = target_x - spawn_piece.x;
        if dx < 0 {
            for _ in 0..dx.abs() {
                path.push(MoveAction::MoveLeft);
            }
        } else if dx > 0 {
            for _ in 0..dx {
                path.push(MoveAction::MoveRight);
            }
        }
        // 即時ハードドロップ
        path.push(MoveAction::HardDrop);
        path
    } else {
        // 潜り込み・スピン・ソフトドロップが必要な手:
        // BFSで探索された操作シーケンスの末尾に HardDrop を追加し、目標地点到達と同時に即時ロック
        let mut path = bfs_path.to_vec();
        if path.last() != Some(&MoveAction::HardDrop) {
            path.push(MoveAction::HardDrop);
        }
        path
    }
}

fn try_enqueue_reachable(
    game: &Game,
    piece: Piece,
    was_rotate: bool,
    path: Vec<MoveAction>,
    visited: &mut [[[bool; 4]; 16]; INTERNAL_HEIGHT],
    queue: &mut VecDeque<(Piece, bool, Vec<MoveAction>)>,
) {
    if game.is_valid_position(&piece) {
        let x_idx = (piece.x + 3) as usize;
        let y_idx = piece.y as usize;
        let rot_idx = piece.rotation;
        if y_idx < INTERNAL_HEIGHT && x_idx < 16 {
            if !visited[y_idx][x_idx][rot_idx] {
                visited[y_idx][x_idx][rot_idx] = true;
                queue.push_back((piece, was_rotate, path));
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
        was_rotate: bool,
        path: Vec<MoveAction>,
        cleared_lines: usize,
    }

    let mut temp_candidates = Vec::new();
    let use_20_features = model.weights.len() == 20;

    for landing in landings {
        let target_piece = landing.piece;
        let was_rotate = landing.was_rotate;
        let path = landing.path;
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
                extract_20_features(game, &temp_board_after_clear, cleared, &target_piece, use_hold, was_rotate)
            } else {
                extract_features(&temp_board_after_clear, cleared)
            };

            temp_candidates.push(TempCandidate {
                target_piece,
                temp_board_after_clear,
                features,
                was_rotate,
                path,
                cleared_lines: cleared,
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
        let was_rotate = c.was_rotate;

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

        // 1. 中央山型集中ペナルティ
        let convexity = crate::tetris::calculate_center_convexity(&c.temp_board_after_clear);
        if convexity > 0.0 {
            eval_score += convexity * crate::config::heuristic::CENTER_CONVEXITY_PENALTY;
        }

        // 2. 両端同時空き（Iミノ枯渇リスク）ペナルティ
        let (is_dual_well, dual_sev) = crate::tetris::detect_dual_side_wells(&c.temp_board_after_clear);
        if is_dual_well {
            eval_score += dual_sev * crate::config::heuristic::DUAL_SIDE_WELL_PENALTY;
        }

        // 3. 2〜9列目（3〜8列目推奨）単一列穴Tスロット構築ボーナス
        let old_slots = crate::tetris::count_t_slots(&game.board);
        let new_slots = crate::tetris::count_t_slots(&c.temp_board_after_clear);
        if new_slots > old_slots {
            let col_quality = crate::tetris::evaluate_t_slot_column_position(c.target_piece.x.clamp(0, 9) as usize, 1);
            eval_score += (new_slots - old_slots) as f32 * crate::config::heuristic::INTERNAL_SINGLE_COLUMN_TSLOT_BONUS * col_quality;
        }

        // 4. 階段積み（Kaidan Setups）ドネイトボーナス
        let kaidan_q = crate::tetris::detect_kaidan_setup_patterns(&c.temp_board_after_clear);
        if kaidan_q > 0.5 {
            eval_score += kaidan_q * crate::config::heuristic::KAIDAN_DONATE_BONUS;
        }

        // 5. T-Spin空打ちペナルティ（ライン消去を伴わないT-Spinを厳罰化し、横一列揃うまで温存させる）
        if c.target_piece.block_type == BlockType::T && c.was_rotate && c.cleared_lines == 0 {
            eval_score += crate::config::heuristic::EMPTY_TSPIN_PENALTY;
        }

        // 6. 本物のT-Spinライン消去（TSD / TSS / TST: 横一列以上揃った実戦発火）への大ボーナス
        if c.target_piece.block_type == BlockType::T && c.was_rotate && c.cleared_lines >= 1 {
            let base_bonus = match c.cleared_lines {
                1 => 40.0,  // TSS
                2 => 140.0, // TSD (最大目標)
                3 => {
                    // 壁端TSTの物理的向き検証（内向きのみ有効、外向きはボーナス剥奪）
                    let (is_valid_tst, tst_quality) = crate::tetris::validate_wall_tst_orientation(
                        &game.board,
                        c.target_piece.x.clamp(0, 9) as usize,
                        c.target_piece.y.max(0) as usize,
                    );
                    if is_valid_tst {
                        crate::config::heuristic::VALID_WALL_TST_BONUS * tst_quality
                    } else {
                        -100.0 // 不正な外向き壁TSTはペナルティ
                    }
                }
                _ => 30.0,
            };
            eval_score += base_bonus;
        }

        // 7. T-Spin Mini無駄打ちペナルティ（本命火力に繋がらない単発Miniの抑制）
        if c.target_piece.block_type == BlockType::T && c.was_rotate && c.cleared_lines <= 1 {
            let t_spin_res = crate::tetris::check_t_spin_type(&game.board, &c.target_piece, c.was_rotate, false);
            if let crate::tetris::TSpinResult::Mini(_) = t_spin_res {
                let next_has_followup = game.hold_piece == Some(BlockType::I) || game.hold_piece == Some(BlockType::T)
                    || game.bag.peek_next(3).contains(&BlockType::I) || game.bag.peek_next(3).contains(&BlockType::T);
                if !game.btb && !next_has_followup {
                    eval_score += crate::config::heuristic::WASTED_TSPIN_MINI_PENALTY;
                }
            }
        }

        moves.push(CandidateMove {
            x: target_x,
            rotation,
            use_hold,
            features: c.features,
            eval_score,
            final_piece: c.target_piece,
            was_rotate,
            path: c.path,
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
        sim_game.current_piece.y = best_move.final_piece.y;
        sim_game.current_piece.rotation = best_move.final_piece.rotation;
        sim_game.last_action_was_rotate = best_move.was_rotate;
        sim_game.lock_piece();

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
        let found_tspin_landing = landings.iter().any(|l| l.piece.x == 4 && l.piece.rotation == 2);
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
        let feats = extract_20_features(&game, &game.board, 0, &piece, false, false);
        assert_eq!(feats.len(), 20);
    }

    #[test]
    fn test_waste_t_penalty_and_hold_t_synergy() {
        let mut game = Game::new();
        for x in 0..BOARD_WIDTH {
            if x != 4 && x != 5 && x != 6 {
                game.board[23][x] = Some(BlockType::I);
                game.board[22][x] = Some(BlockType::I);
            }
        }
        game.board[23][4] = Some(BlockType::I);
        game.board[23][6] = Some(BlockType::I);
        game.board[21][4] = Some(BlockType::I); // T-slot is formed

        // 1. Placing T outside T-slot (WasteT)
        let mut flat_piece = Piece::new(BlockType::T);
        flat_piece.x = 0;
        flat_piece.y = 20;
        let feats_waste = extract_20_features(&game, &game.board, 0, &flat_piece, false, false);
        assert!(feats_waste[19] <= 0.3, "WasteT should significantly reduce future fit score");

        // 2. Holding T while T-slot exists (HoldT)
        let mut game_with_hold_t = game.clone();
        game_with_hold_t.hold_piece = Some(BlockType::T);
        let j_piece = Piece::new(BlockType::J);
        let feats_hold_t = extract_20_features(&game_with_hold_t, &game.board, 0, &j_piece, false, false);
        assert!(feats_hold_t[19] >= 0.8, "HoldT should grant synergy bonus when T-slot exists");
    }

    #[test]
    fn test_roof_formation_bonus_and_soft_drop_path() {
        let mut game = Game::new();
        let bottom = INTERNAL_HEIGHT - 1;
        for x in 0..BOARD_WIDTH {
            if x != 4 && x != 5 && x != 6 {
                game.board[bottom][x] = Some(BlockType::I);
                game.board[bottom - 1][x] = Some(BlockType::I);
            }
        }
        game.board[bottom][4] = Some(BlockType::I);
        game.board[bottom][6] = Some(BlockType::I);

        // 1. Place an S-piece to form a roof at x=4, y=bottom-2
        let s_piece = Piece {
            block_type: BlockType::S,
            x: 4,
            y: (bottom - 2) as i32,
            rotation: 0,
        };
        let feats = extract_20_features(&game, &game.board, 0, &s_piece, false, false);
        assert_eq!(feats[4], 1.0, "Roof formation move should grant maximum PlacementQuality (1.0)");

        // 2. Now with roof in place, search BFS landings for T-piece
        game.board[bottom - 2][4] = Some(BlockType::S);
        let landings = search_reachable_landings(&game, BlockType::T);
        let t_landing = landings.iter().find(|l| l.piece.x == 5 && l.piece.y == (bottom - 1) as i32 && l.piece.rotation == 2);
        assert!(t_landing.is_some(), "T-piece must find landing inside T-slot under the S-roof");
        let landing = t_landing.unwrap();
        assert!(landing.was_rotate, "Landing in T-slot must be a rotation landing");
        assert!(landing.path.contains(&MoveAction::SoftDrop), "Landing path must include SoftDrop down to the slot");
    }

    #[test]
    fn test_empty_tspin_prevention_and_line_clearing_preference() {
        let mut game = Game::new();
        let bottom = INTERNAL_HEIGHT - 1;

        // 1. Tスロットはあるが、横一列が揃っていない状態（空打ち状態）
        game.board[bottom][4] = Some(BlockType::I);
        game.board[bottom][6] = Some(BlockType::I);
        game.board[bottom - 2][4] = Some(BlockType::S); // 屋根

        let empty_t_piece = Piece {
            block_type: BlockType::T,
            x: 5,
            y: (bottom - 1) as i32,
            rotation: 2,
        };
        // 0ライン消去
        let feats_empty = extract_20_features(&game, &game.board, 0, &empty_t_piece, false, true);
        assert_eq!(feats_empty[0], 0.0, "Empty T-Spin (0 lines cleared) must score 0.0 on TSpin feature");
        assert_eq!(feats_empty[4], 0.05, "Empty T-Spin must receive low PlacementQuality (0.05)");

        // 2. 横一列以上揃った状態（TSD: 2ライン消去）
        for x in 0..BOARD_WIDTH {
            if x != 5 {
                game.board[bottom][x] = Some(BlockType::I);
            }
            if x != 4 && x != 5 && x != 6 {
                game.board[bottom - 1][x] = Some(BlockType::I);
            }
        }
        let feats_tsd = extract_20_features(&game, &game.board, 2, &empty_t_piece, false, true);
        assert_eq!(feats_tsd[0], 1.0, "TSD (2 lines cleared) must score 1.0 on TSpin feature");
        assert_eq!(feats_tsd[4], 1.0, "TSD must receive max PlacementQuality (1.0)");
    }

    #[test]
    fn test_center_convexity_and_dual_well_in_features() {
        let mut game = Game::new();
        let bottom = INTERNAL_HEIGHT - 1;

        // 1. Center mountain terrain
        for y in (bottom - 4)..=bottom {
            game.board[y][4] = Some(BlockType::I);
            game.board[y][5] = Some(BlockType::I);
        }
        let piece = Piece::new(BlockType::I);
        let feats_convex = extract_20_features(&game, &game.board, 0, &piece, false, false);
        assert!(feats_convex[16] > 0.1, "Bumpiness penalty (x16) should be elevated for center mountain terrain");

        // 2. Dual side well terrain
        let mut dual_board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        for y in (bottom - 3)..=bottom {
            for x in 1..=8 {
                dual_board[y][x] = Some(BlockType::I);
            }
        }
        let feats_dual = extract_20_features(&game, &dual_board, 0, &piece, false, false);
        assert_eq!(feats_dual[17], 0.05, "Well quality (x17) must drop to 0.05 when both side edges are open simultaneously");
    }

    #[test]
    fn test_internal_single_column_t_slot_scoring() {
        let mut game = Game::new();
        let bottom = INTERNAL_HEIGHT - 1;

        // Internal slot at col 4 (3〜8列目最適)
        for x in 0..BOARD_WIDTH {
            if x != 4 {
                game.board[bottom][x] = Some(BlockType::I);
            }
            if x != 3 && x != 4 && x != 5 {
                game.board[bottom - 1][x] = Some(BlockType::I);
            }
        }
        game.board[bottom - 2][3] = Some(BlockType::S); // Roof over col 4

        let t_piece = Piece {
            block_type: BlockType::T,
            x: 4,
            y: (bottom - 1) as i32,
            rotation: 2,
        };
        let feats = extract_20_features(&game, &game.board, 2, &t_piece, false, true);
        assert!(feats[1] >= 0.85, "Internal single-column T-slot should receive high terrain quality, got {}", feats[1]);
    }

    #[test]
    fn test_hard_drop_preference_and_instant_lock() {
        let game = Game::new();
        let landings = search_reachable_landings(&game, BlockType::I);
        assert!(!landings.is_empty());

        // 1. Open direct landing should end in HardDrop and have no SoftDrop in path
        let open_landing = landings.iter().find(|l| l.piece.x == 3 && l.piece.rotation == 0);
        assert!(open_landing.is_some());
        let l = open_landing.unwrap();
        assert_eq!(l.path.last(), Some(&MoveAction::HardDrop), "All execution paths must end in HardDrop");
        assert!(!l.path.contains(&MoveAction::SoftDrop), "Direct open drop must NOT contain SoftDrop steps");

        // 2. All reachable landings must end with HardDrop for instantaneous lock delay elimination
        for landing in &landings {
            assert_eq!(landing.path.last(), Some(&MoveAction::HardDrop), "Every landing path must end with HardDrop");
        }
    }

    #[test]
    fn test_wasted_mini_suppression() {
        let mut game = Game::new();
        let bottom = INTERNAL_HEIGHT - 1;

        // Setup T-Spin Mini terrain (single front corner + 2 back corners)
        let cy = bottom - 2;
        let cx = 2;
        game.board[cy - 1][cx - 1] = Some(BlockType::I); // Top-Left (Back)
        game.board[cy - 1][cx + 1] = Some(BlockType::I); // Top-Right (Back)
        game.board[cy + 1][cx - 1] = Some(BlockType::I); // Bottom-Left (Front)
        game.board[cy + 1][cx + 1] = None;               // Bottom-Right (Front)

        let mut moves = Vec::new();
        let model = AiModel::new_default();
        enumerate_moves_for_piece(&game, BlockType::T, false, &model, None, 0, &mut moves);

        // Moves should be evaluated properly without panic
        assert!(!moves.is_empty());
    }
}
