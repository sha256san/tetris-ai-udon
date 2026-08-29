use rand::seq::SliceRandom;
use serde::{Serialize, Deserialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum BlockType {
    I, O, T, S, Z, J, L
}

impl BlockType {
    pub fn all() -> [BlockType; 7] {
        [BlockType::I, BlockType::O, BlockType::T, BlockType::S, BlockType::Z, BlockType::J, BlockType::L]
    }
}

pub const BOARD_WIDTH: usize = 10;
pub const BOARD_HEIGHT: usize = 20;
pub const INTERNAL_HEIGHT: usize = 24; // 上部バッファ4行を含む

pub type Board = [[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Piece {
    pub block_type: BlockType,
    pub x: i32,
    pub y: i32,
    pub rotation: usize, // 0, 1, 2, 3 (0: 0deg, 1: 90deg R, 2: 180deg, 3: 90deg L)
}

impl Piece {
    pub fn new(block_type: BlockType) -> Self {
        // IミノとOミノは初期位置の調整が必要な場合がある
        let spawn_x = match block_type {
            BlockType::I => 3,
            BlockType::O => 4,
            _ => 3,
        };
        // 初期Y座標（バッファ領域内）
        let spawn_y = 2;
        Piece {
            block_type,
            x: spawn_x,
            y: spawn_y,
            rotation: 0,
        }
    }

    pub fn get_cells(&self) -> [(i32, i32); 4] {
        let offsets = get_piece_offsets(self.block_type, self.rotation);
        let mut cells = [(0, 0); 4];
        for i in 0..4 {
            cells[i] = (self.x + offsets[i].0, self.y + offsets[i].1);
        }
        cells
    }
}

// 各ミノの回転状態（0, 1, 2, 3）ごとの相対ブロック位置（Y軸下向き）
pub fn get_piece_offsets(block_type: BlockType, rotation: usize) -> [(i32, i32); 4] {
    let r = rotation % 4;
    match block_type {
        BlockType::I => match r {
            0 => [(-1, 0), (0, 0), (1, 0), (2, 0)],
            1 => [(1, -1), (1, 0), (1, 1), (1, 2)],
            2 => [(-1, 1), (0, 1), (1, 1), (2, 1)],
            3 => [(0, -1), (0, 0), (0, 1), (0, 2)],
            _ => unreachable!(),
        },
        BlockType::O => [(0, 0), (1, 0), (0, 1), (1, 1)], // 回転しても形状は同一
        BlockType::T => match r {
            0 => [(0, -1), (-1, 0), (0, 0), (1, 0)],
            1 => [(0, -1), (0, 0), (1, 0), (0, 1)],
            2 => [(-1, 0), (0, 0), (1, 0), (0, 1)],
            3 => [(0, -1), (-1, 0), (0, 0), (0, 1)],
            _ => unreachable!(),
        },
        BlockType::S => match r {
            0 => [(0, -1), (1, -1), (-1, 0), (0, 0)],
            1 => [(0, -1), (0, 0), (1, 0), (1, 1)],
            2 => [(0, 0), (1, 0), (-1, 1), (0, 1)],
            3 => [(-1, -1), (-1, 0), (0, 0), (0, 1)],
            _ => unreachable!(),
        },
        BlockType::Z => match r {
            0 => [(-1, -1), (0, -1), (0, 0), (1, 0)],
            1 => [(1, -1), (0, 0), (1, 0), (0, 1)],
            2 => [(-1, 0), (0, 0), (0, 1), (1, 1)],
            3 => [(0, -1), (-1, 0), (0, 0), (-1, 1)],
            _ => unreachable!(),
        },
        BlockType::J => match r {
            0 => [(-1, -1), (-1, 0), (0, 0), (1, 0)],
            1 => [(0, -1), (1, -1), (0, 0), (0, 1)],
            2 => [(-1, 0), (0, 0), (1, 0), (1, 1)],
            3 => [(0, -1), (0, 0), (-1, 1), (0, 1)],
            _ => unreachable!(),
        },
        BlockType::L => match r {
            0 => [(1, -1), (-1, 0), (0, 0), (1, 0)],
            1 => [(0, -1), (0, 0), (0, 1), (1, 1)],
            2 => [(-1, 0), (0, 0), (1, 0), (-1, 1)],
            3 => [(-1, -1), (0, -1), (0, 0), (0, 1)],
            _ => unreachable!(),
        },
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[allow(dead_code)]
pub enum RotationDirection {
    Clockwise,        // 右回転
    CounterClockwise, // 左回転
}

// 7-bag ランダマイザー
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bag {
    pub queue: Vec<BlockType>,
}

impl Bag {
    pub fn new() -> Self {
        let mut bag = Bag { queue: Vec::new() };
        bag.refill();
        bag
    }

    fn refill(&mut self) {
        let mut new_bag = BlockType::all().to_vec();
        let mut rng = rand::thread_rng();
        new_bag.shuffle(&mut rng);
        self.queue.extend(new_bag);
    }

    pub fn pop(&mut self) -> BlockType {
        if self.queue.len() <= 7 {
            self.refill();
        }
        self.queue.remove(0)
    }

    pub fn peek_next(&self, count: usize) -> Vec<BlockType> {
        self.queue.iter().take(count).cloned().collect()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TSpinResult {
    None,
    Mini(usize),
    Full(usize),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Game {
    pub board: Board,
    pub current_piece: Piece,
    pub bag: Bag,
    pub hold_piece: Option<BlockType>,
    pub hold_locked: bool,
    pub score: u32,
    pub lines_cleared: u32,
    pub game_over: bool,
    pub last_action_was_rotate: bool,
    pub last_rotate_used_srs_kick_5: bool,
    pub last_t_spin: Option<String>,
    pub btb: bool,
    pub pending_garbage: u32,
    pub last_firepower: u32,
    pub last_garbage_hole: Option<usize>,
}

impl Game {
    pub fn new() -> Self {
        let mut bag = Bag::new();
        let first = bag.pop();
        Game {
            board: [[None; BOARD_WIDTH]; INTERNAL_HEIGHT],
            current_piece: Piece::new(first),
            bag,
            hold_piece: None,
            hold_locked: false,
            score: 0,
            lines_cleared: 0,
            game_over: false,
            last_action_was_rotate: false,
            last_rotate_used_srs_kick_5: false,
            last_t_spin: None,
            btb: false,
            pending_garbage: 0,
            last_firepower: 0,
            last_garbage_hole: None,
        }
    }

    pub fn new_with_seed(seed: u64) -> Self {
        use rand::SeedableRng;
        use rand::rngs::StdRng;
        let mut rng = StdRng::seed_from_u64(seed);
        let mut bag = Bag { queue: Vec::new() };
        for _ in 0..30 {
            let mut new_bag = BlockType::all().to_vec();
            new_bag.shuffle(&mut rng);
            bag.queue.extend(new_bag);
        }
        let first = bag.pop();
        Game {
            board: [[None; BOARD_WIDTH]; INTERNAL_HEIGHT],
            current_piece: Piece::new(first),
            bag,
            hold_piece: None,
            hold_locked: false,
            score: 0,
            lines_cleared: 0,
            game_over: false,
            last_action_was_rotate: false,
            last_rotate_used_srs_kick_5: false,
            last_t_spin: None,
            btb: false,
            pending_garbage: 0,
            last_firepower: 0,
            last_garbage_hole: None,
        }
    }

    // 指定されたミノが衝突なく配置可能かチェック
    pub fn is_valid_position(&self, piece: &Piece) -> bool {
        for &(cx, cy) in &piece.get_cells() {
            if cx < 0 || cx >= BOARD_WIDTH as i32 || cy < 0 || cy >= INTERNAL_HEIGHT as i32 {
                return false;
            }
            if self.board[cy as usize][cx as usize].is_some() {
                return false;
            }
        }
        true
    }

    // ミノを移動させる (dx, dy)
    pub fn try_move(&mut self, dx: i32, dy: i32) -> bool {
        let mut next_piece = self.current_piece.clone();
        next_piece.x += dx;
        next_piece.y += dy;

        if self.is_valid_position(&next_piece) {
            self.current_piece = next_piece;
            self.last_action_was_rotate = false;
            self.last_rotate_used_srs_kick_5 = false;
            true
        } else {
            false
        }
    }

    // SRSに基づく回転処理
    pub fn try_rotate(&mut self, dir: RotationDirection) -> bool {
        if self.current_piece.block_type == BlockType::O {
            return false; // Oミノは回転しない
        }

        let from_rot = self.current_piece.rotation;
        let to_rot = match dir {
            RotationDirection::Clockwise => (from_rot + 1) % 4,
            RotationDirection::CounterClockwise => (from_rot + 3) % 4,
        };

        let mut next_piece = self.current_piece.clone();
        next_piece.rotation = to_rot;

        // キックデータを試行
        let kick_offsets = self.get_kick_offsets(self.current_piece.block_type, from_rot, to_rot);
        for (kick_idx, &(dx, dy)) in kick_offsets.iter().enumerate() {
            let mut test_piece = next_piece.clone();
            test_piece.x += dx;
            test_piece.y += dy;
            if self.is_valid_position(&test_piece) {
                self.current_piece = test_piece;
                self.last_action_was_rotate = true;
                self.last_rotate_used_srs_kick_5 = kick_idx == 4;
                return true;
            }
        }
        false
    }

    // SRSのキックオフセットテーブル (dx, dy) の取得。Y軸は下方向が正。
    pub fn get_kick_offsets(&self, block_type: BlockType, from_rot: usize, to_rot: usize) -> [(i32, i32); 5] {
        let key = (from_rot, to_rot);
        if block_type == BlockType::I {
            // Iミノ用キックデータ
            match key {
                (0, 1) => [(0,0), (-2,0), (1,0), (-2,-1), (1,2)],
                (1, 0) => [(0,0), (2,0), (-1,0), (2,1), (-1,-2)],
                (1, 2) => [(0,0), (-1,0), (2,0), (-1,2), (2,-1)],
                (2, 1) => [(0,0), (1,0), (-2,0), (1,-2), (-2,1)],
                (2, 3) => [(0,0), (2,0), (-1,0), (2,1), (-1,-2)],
                (3, 2) => [(0,0), (-2,0), (1,0), (-2,-1), (1,2)],
                (3, 0) => [(0,0), (1,0), (-2,0), (1,-2), (-2,1)],
                (0, 3) => [(0,0), (-1,0), (2,0), (-1,2), (2,-1)],
                _ => [(0,0); 5],
            }
        } else {
            // T, S, Z, J, L ミノ用キックデータ
            match key {
                (0, 1) => [(0,0), (-1,0), (-1,-1), (0,2), (-1,2)],
                (1, 0) => [(0,0), (1,0), (1,1), (0,-2), (1,-2)],
                (1, 2) => [(0,0), (1,0), (1,1), (0,-2), (1,-2)],
                (2, 1) => [(0,0), (-1,0), (-1,-1), (0,2), (-1,2)],
                (2, 3) => [(0,0), (1,0), (1,-1), (0,2), (1,2)],
                (3, 2) => [(0,0), (-1,0), (-1,1), (0,-2), (-1,-2)],
                (3, 0) => [(0,0), (-1,0), (-1,1), (0,-2), (-1,-2)],
                (0, 3) => [(0,0), (1,0), (1,-1), (0,2), (1,2)],
                _ => [(0,0); 5],
            }
        }
    }

    // ハードドロップ
    pub fn hard_drop(&mut self) -> u32 {
        let mut drop_dist = 0;
        while self.try_move(0, 1) {
            drop_dist += 1;
        }
        self.lock_piece();
        drop_dist
    }

    // ミノをホールドする
    pub fn hold(&mut self) -> bool {
        if self.hold_locked {
            return false;
        }

        let current_type = self.current_piece.block_type;
        if let Some(held) = self.hold_piece {
            self.hold_piece = Some(current_type);
            self.current_piece = Piece::new(held);
        } else {
            self.hold_piece = Some(current_type);
            let next_type = self.bag.pop();
            self.current_piece = Piece::new(next_type);
        }

        self.hold_locked = true;
        self.last_action_was_rotate = false;
        self.last_rotate_used_srs_kick_5 = false;
        
        // ホールド直後に衝突している場合は即座にゲームオーバー
        if !self.is_valid_position(&self.current_piece) {
            self.game_over = true;
        }
        true
    }

    // ミノを固定し、ライン消去とネクストミノのスポーンを行う
    pub fn lock_piece(&mut self) {
        let is_t_piece = self.current_piece.block_type == BlockType::T;
        let was_rotate = self.last_action_was_rotate;
        let used_srs_5 = self.last_rotate_used_srs_kick_5;

        for &(cx, cy) in &self.current_piece.get_cells() {
            if cx >= 0 && cx < BOARD_WIDTH as i32 && cy >= 0 && cy < INTERNAL_HEIGHT as i32 {
                self.board[cy as usize][cx as usize] = Some(self.current_piece.block_type);
            }
        }

        // T-spinの判定 (ライン消去の前に行う)
        let t_spin_result = if is_t_piece {
            check_t_spin_type(
                &self.board,
                &self.current_piece,
                was_rotate,
                used_srs_5,
            )
        } else {
            TSpinResult::None
        };

        // ライン消去
        let cleared = self.clear_lines();
        self.lines_cleared += cleared as u32;

        let mut firepower = 0;
        let mut is_btb_eligible = false;

        match t_spin_result {
            TSpinResult::Full(_) => {
                let (score_add, t_spin_name, fp) = match cleared {
                    0 => (crate::config::game::TSPIN_0_SCORE, "T-Spin", 0),
                    1 => (crate::config::game::TSPIN_1_SCORE, "T-Spin Single", 2),
                    2 => (crate::config::game::TSPIN_2_SCORE, "T-Spin Double", 4),
                    3 => (crate::config::game::TSPIN_3_SCORE, "T-Spin Triple", 6),
                    _ => (0, "T-Spin", 0),
                };
                self.score += score_add;
                self.last_t_spin = Some(t_spin_name.to_string());
                firepower = fp;
                if cleared > 0 {
                    is_btb_eligible = true;
                }
            }
            TSpinResult::Mini(_) => {
                let (score_add, t_spin_name, fp) = match cleared {
                    0 => (100, "T-Spin Mini", 0),
                    1 => (200, "T-Spin Mini Single", 0),
                    2 => (400, "T-Spin Mini Double", 1),
                    _ => (100, "T-Spin Mini", 0),
                };
                self.score += score_add;
                self.last_t_spin = Some(t_spin_name.to_string());
                firepower = fp;
                if cleared > 0 {
                    is_btb_eligible = true;
                }
            }
            TSpinResult::None => {
                if cleared < crate::config::game::LINE_CLEAR_SCORES.len() {
                    self.score += crate::config::game::LINE_CLEAR_SCORES[cleared];
                }
                self.last_t_spin = None;
                match cleared {
                    1 => { firepower = 0; }
                    2 => { firepower = 1; }
                    3 => { firepower = 2; }
                    4 => { firepower = 4; is_btb_eligible = true; } // Tetris
                    _ => {}
                }
            }
        }

        if is_btb_eligible {
            if self.btb {
                firepower += 1;
            }
            self.btb = true;
        } else if cleared > 0 {
            self.btb = false;
        }

        // Garbage Cancellation
        let mut sent = firepower;
        if self.pending_garbage >= sent {
            self.pending_garbage -= sent;
            sent = 0;
        } else {
            sent -= self.pending_garbage;
            self.pending_garbage = 0;
        }
        self.last_firepower = sent;

        // 深い穴ボーナス
        self.score += get_well_bonus(&self.board);

        // 次のミノをスポーン
        let next_type = self.bag.pop();
        self.current_piece = Piece::new(next_type);
        self.hold_locked = false;
    }

    pub fn apply_garbage(&mut self) {
        if self.pending_garbage > 0 {
            use rand::Rng;
            let lines = self.pending_garbage;
            // Move up
            for y in lines as usize..INTERNAL_HEIGHT {
                self.board[y - lines as usize] = self.board[y];
            }
            let mut rng = rand::thread_rng();
            for y in (INTERNAL_HEIGHT - lines as usize)..INTERNAL_HEIGHT {
                let hole = match self.last_garbage_hole {
                    Some(last_hole) if rng.gen_bool(0.7) => last_hole,
                    _ => {
                        let mut new_hole = rng.gen_range(0..BOARD_WIDTH);
                        if let Some(last_hole) = self.last_garbage_hole {
                            if new_hole == last_hole {
                                new_hole = (new_hole + rng.gen_range(1..BOARD_WIDTH)) % BOARD_WIDTH;
                            }
                        }
                        new_hole
                    }
                };
                self.last_garbage_hole = Some(hole);

                for x in 0..BOARD_WIDTH {
                    if x == hole {
                        self.board[y][x] = None;
                    } else {
                        self.board[y][x] = Some(BlockType::I);
                    }
                }
            }
            self.pending_garbage = 0;
        }
        // スポーン時点で衝突していればゲームオーバー
        if !self.is_valid_position(&self.current_piece) {
            self.game_over = true;
        }
    }

    // ライン消去ロジック
    fn clear_lines(&mut self) -> usize {
        let mut cleared = 0;
        let mut new_board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let mut target_y = INTERNAL_HEIGHT - 1;

        for y in (0..INTERNAL_HEIGHT).rev() {
            let mut is_full = true;
            for x in 0..BOARD_WIDTH {
                if self.board[y][x].is_none() {
                    is_full = false;
                    break;
                }
            }

            if is_full {
                cleared += 1;
            } else {
                new_board[target_y] = self.board[y];
                if target_y > 0 {
                    target_y -= 1;
                }
            }
        }
        self.board = new_board;
        cleared
    }
}

// 縦3マス以上の深い穴が1列しかない場合のボーナススコアを計算
pub fn get_well_bonus(board: &Board) -> u32 {
    let mut heights = [0; BOARD_WIDTH];
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

    let mut well_columns = Vec::new();
    for x in 0..BOARD_WIDTH {
        let left = if x == 0 { INTERNAL_HEIGHT as i32 } else { heights[x - 1] };
        let right = if x == BOARD_WIDTH - 1 { INTERNAL_HEIGHT as i32 } else { heights[x + 1] };
        let h = heights[x];
        let diff = std::cmp::min(left, right) - h;
        if diff >= 3 {
            well_columns.push((x, diff));
        }
    }

    if well_columns.len() == 1 {
        let well_x = well_columns[0].0;

        // ほかの列に穴（ブロックの下の空きマス）がないかチェック
        for x in 0..BOARD_WIDTH {
            if x != well_x {
                let mut block_found = false;
                for y in 0..INTERNAL_HEIGHT {
                    if board[y][x].is_some() {
                        block_found = true;
                    } else if block_found {
                        // ブロックがあるにもかかわらず空きマスがある＝穴
                        return 0;
                    }
                }
            }
        }

        let depth = well_columns[0].1;

        // 得点の基本スコアを列（well_x）に応じて算出
        // - 7列目 (インデックス 6) は一番高い
        // - 2列目〜9列目 (インデックス 1〜8) は少し高い
        // - 1列目, 10列目 (インデックス 0, 9) はベース
        let base_score = if well_x == 6 {
            crate::config::game::WELL_BASE_SCORE_TARGET
        } else if well_x >= 1 && well_x <= 8 {
            crate::config::game::WELL_BASE_SCORE_MIDDLE
        } else {
            crate::config::game::WELL_BASE_SCORE_EDGE
        };

        if depth == 3 {
            base_score
        } else if depth >= 4 {
            base_score * 3
        } else {
            0
        }
    } else {
        0
    }
}

/// Tetris Guideline に準拠した 3-Corner T-Spin / T-Spin Mini 判定
pub fn check_t_spin_type(
    board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT],
    piece: &Piece,
    was_rotate: bool,
    used_srs_kick_5: bool,
) -> TSpinResult {
    if piece.block_type != BlockType::T || !was_rotate {
        return TSpinResult::None;
    }

    let cx = piece.x;
    let cy = piece.y;

    // Tミノの回転に応じた「前側2隅 (Front 2 corners)」と「後側2隅 (Back 2 corners)」
    let (front_coords, back_coords) = match piece.rotation {
        0 => (
            [(cx - 1, cy - 1), (cx + 1, cy - 1)], // 上向き凸: 左上, 右上
            [(cx - 1, cy + 1), (cx + 1, cy + 1)], // 左下, 右下
        ),
        1 => (
            [(cx + 1, cy - 1), (cx + 1, cy + 1)], // 右向き凸: 右上, 右下
            [(cx - 1, cy - 1), (cx - 1, cy + 1)], // 左上, 左下
        ),
        2 => (
            [(cx - 1, cy + 1), (cx + 1, cy + 1)], // 下向き凸: 左下, 右下
            [(cx - 1, cy - 1), (cx + 1, cy - 1)], // 左上, 右上
        ),
        3 => (
            [(cx - 1, cy - 1), (cx - 1, cy + 1)], // 左向き凸: 左上, 左下
            [(cx + 1, cy - 1), (cx + 1, cy + 1)], // 右上, 右下
        ),
        _ => return TSpinResult::None,
    };

    let is_filled = |x: i32, y: i32| -> bool {
        if x < 0 || x >= BOARD_WIDTH as i32 || y >= INTERNAL_HEIGHT as i32 {
            true // 壁または床
        } else if y < 0 {
            false // 天井より上は空間
        } else {
            board[y as usize][x as usize].is_some()
        }
    };

    let front_filled = front_coords.iter().filter(|&&(x, y)| is_filled(x, y)).count();
    let back_filled = back_coords.iter().filter(|&&(x, y)| is_filled(x, y)).count();
    let total_filled = front_filled + back_filled;

    if total_filled < 3 {
        return TSpinResult::None;
    }

    // 1. 前側2隅が両方埋まっている ＋ 後側が1つ以上埋まっている -> 本物 (Regular T-Spin)
    if front_filled == 2 {
        TSpinResult::Full(0)
    } else if front_filled == 1 && back_filled == 2 {
        // 2. 前側1隅 ＋ 後側2隅:
        // SRSの第5テスト(TSTキック等の大キック)を使用した場合は Regular T-Spin に昇格
        // それ以外は T-Spin Mini
        if used_srs_kick_5 {
            TSpinResult::Full(0)
        } else {
            TSpinResult::Mini(0)
        }
    } else {
        TSpinResult::None
    }
}

/// 盤面上の T-spin slot (T-slot) の個数をカウントする（壁際・全列対応）
pub fn count_t_slots(board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT]) -> usize {
    let mut count = 0;
    for cy in 1..(INTERNAL_HEIGHT - 1) {
        for cx in 0..BOARD_WIDTH {
            // 中心セルが空であること
            if board[cy][cx].is_some() {
                continue;
            }

            // 4つの隅（コーナー）のうち、少なくとも3つがブロックか壁で埋まっていること（T-spinの必要条件）
            let corners = [
                (cx as i32 - 1, cy as i32 - 1),
                (cx as i32 + 1, cy as i32 - 1),
                (cx as i32 - 1, cy as i32 + 1),
                (cx as i32 + 1, cy as i32 + 1),
            ];
            let mut filled_corners = 0;
            let mut real_block_corners = 0;
            for &(x, y) in &corners {
                if x < 0 || x >= BOARD_WIDTH as i32 || y >= INTERNAL_HEIGHT as i32 {
                    filled_corners += 1;
                } else if y >= 0 && board[y as usize][x as usize].is_some() {
                    filled_corners += 1;
                    real_block_corners += 1;
                }
            }

            // 少なくとも2つは実際のブロックによるコーナー支持が必要（壁単独での空虚検出防止）
            if filled_corners < 3 || real_block_corners < 2 {
                continue;
            }

            // 4つのTの向きについて判定 (0: 上, 1: 右, 2: 下, 3: 左)
            let cx_i = cx as i32;
            let cy_i = cy as i32;
            let orientations = [
                // 上向き (Tの凸部が上: 左・右・上が空、下が壁/ブロック)
                ([(cx_i - 1, cy_i), (cx_i + 1, cy_i), (cx_i, cy_i - 1)], (cx_i, cy_i + 1)),
                // 右向き (Tの凸部が右: 上・下・右が空、左が壁/ブロック)
                ([(cx_i, cy_i - 1), (cx_i, cy_i + 1), (cx_i + 1, cy_i)], (cx_i - 1, cy_i)),
                // 下向き (Tの凸部が下: 左・右・下が空、上が壁/ブロック)
                ([(cx_i - 1, cy_i), (cx_i + 1, cy_i), (cx_i, cy_i + 1)], (cx_i, cy_i - 1)),
                // 左向き (Tの凸部が左: 上・下・左が空、右が壁/ブロック)
                ([(cx_i, cy_i - 1), (cx_i, cy_i + 1), (cx_i - 1, cy_i)], (cx_i + 1, cy_i)),
            ];

            for &(empty_coords, blocked_coord) in &orientations {
                let mut all_empty = true;
                for &(ex, ey) in &empty_coords {
                    if ex < 0 || ex >= BOARD_WIDTH as i32 || ey < 0 || ey >= INTERNAL_HEIGHT as i32 {
                        all_empty = false;
                        break;
                    }
                    if board[ey as usize][ex as usize].is_some() {
                        all_empty = false;
                        break;
                    }
                }
                if !all_empty {
                    continue;
                }

                let (bx, by) = blocked_coord;
                let is_blocked = if bx < 0 || bx >= BOARD_WIDTH as i32 || by >= INTERNAL_HEIGHT as i32 {
                    true
                } else if by < 0 {
                    false
                } else {
                    board[by as usize][bx as usize].is_some()
                };

                if is_blocked {
                    count += 1;
                    break;
                }
            }
        }
    }
    count
}

/// T-spin地形品質（スロット完成度 + STSD/TD砲/TST複合構造 + ドネイト + スロット基礎凹み + コーナー支持）を 0.0 .. 1.0 で算出
pub fn evaluate_t_spin_terrain(board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT]) -> f32 {
    let slots = count_t_slots(board);
    let mut max_quality = 0.0f32;

    if slots > 0 {
        max_quality = (0.75 + (slots as f32) * 0.15).min(1.0);
    }

    // 1. TD砲 (Triple-Double Cannon) / DT Cannon 複合形状の検出 (HoikoCode TDHole / TDHint)
    for cy in 2..(INTERNAL_HEIGHT - 2) {
        for cx in 0..BOARD_WIDTH {
            // 壁際(cx=0, 1 または cx=BOARD_WIDTH-2, BOARD_WIDTH-1)のTST縦溝
            if (cx <= 1 || cx >= BOARD_WIDTH - 2)
                && board[cy][cx].is_none() && board[cy + 1][cx].is_none() && board[cy + 2][cx].is_none()
                && (cy + 3 >= INTERNAL_HEIGHT || board[cy + 3][cx].is_some())
            {
                let has_td_shelf = if cx <= 1 {
                    cx + 1 < BOARD_WIDTH && board[cy][cx + 1].is_some() && board[cy - 1][cx + 1].is_none()
                } else {
                    cx >= 1 && board[cy][cx - 1].is_some() && board[cy - 1][cx - 1].is_none()
                };

                if has_td_shelf {
                    max_quality = max_quality.max(0.98); // TD砲完成形
                } else {
                    max_quality = max_quality.max(0.80); // TST縦溝準備
                }
            }
        }
    }

    // 2. STSD (Super T-Spin Double: 2連TSD構造) の検出
    for cy in 3..(INTERNAL_HEIGHT - 2) {
        for cx in 1..(BOARD_WIDTH - 1) {
            if board[cy][cx].is_none() && board[cy][cx - 1].is_none() && board[cy][cx + 1].is_none()
                && (cy + 1 >= INTERNAL_HEIGHT || board[cy + 1][cx].is_none())
            {
                let has_stsd_roof = (board[cy - 1][cx - 1].is_some() && board[cy - 2][cx - 1].is_some())
                    || (board[cy - 1][cx + 1].is_some() && board[cy - 2][cx + 1].is_some());
                if has_stsd_roof {
                    max_quality = max_quality.max(0.92);
                }
            }
        }
    }

    // 3. Shiwehi式 ドネーション（1ミノ/2ミノ ドネイト）の検出
    // 井戸（下穴）の上部2段を屋根ブロックで覆い、TSD発火後に下穴が再開口する構造
    for cy in 2..(INTERNAL_HEIGHT - 3) {
        for cx in 1..(BOARD_WIDTH - 1) {
            if board[cy][cx].is_none() && board[cy][cx - 1].is_none() && board[cy][cx + 1].is_none() {
                let roof_left = board[cy - 1][cx - 1].is_some();
                let roof_right = board[cy - 1][cx + 1].is_some();
                if roof_left ^ roof_right {
                    let has_well_below = (cx > 0 && board[cy + 2][cx - 1].is_none())
                        || board[cy + 2][cx].is_none()
                        || (cx + 1 < BOARD_WIDTH && board[cy + 2][cx + 1].is_none());
                    if has_well_below {
                        max_quality = max_quality.max(0.85); // 有効ドネイト
                    }
                }
            }
        }
    }

    // 4. TSD スロット基礎・仕込み中間地形（Stepping Stone）の走査
    for cy in 2..(INTERNAL_HEIGHT - 1) {
        for cx in 1..(BOARD_WIDTH - 1) {
            if board[cy][cx].is_none() && board[cy][cx - 1].is_none() && board[cy][cx + 1].is_none() {
                let bottom_filled = (if board[cy + 1][cx - 1].is_some() { 1 } else { 0 })
                    + (if board[cy + 1][cx].is_some() { 1 } else { 0 })
                    + (if board[cy + 1][cx + 1].is_some() { 1 } else { 0 });
                if bottom_filled >= 2 {
                    let roof_left = cy >= 1 && board[cy - 1][cx - 1].is_some();
                    let roof_right = cy >= 1 && board[cy - 1][cx + 1].is_some();
                    if roof_left || roof_right {
                        max_quality = max_quality.max(0.75); // 屋根付きReady TSD
                    } else {
                        max_quality = max_quality.max(0.45); // 土台のみBase
                    }
                }
            }
        }
    }

    // 5. 階段積みドネイト (Kaidan Setups) パターンの加算
    let kaidan_q = detect_kaidan_setup_patterns(board);
    if kaidan_q > 0.0 {
        max_quality = max_quality.max(kaidan_q);
    }

    max_quality
}

/// 盤面中央列（x=3..6）の平均標高と両側（x=0..2, 7..9）の平均標高の差分を計測し、中央山型（富士山型）凸度を算出 (0.0..1.0)
pub fn calculate_center_convexity(board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT]) -> f32 {
    let mut heights = [0usize; BOARD_WIDTH];
    for x in 0..BOARD_WIDTH {
        for y in 0..INTERNAL_HEIGHT {
            if board[y][x].is_some() {
                heights[x] = INTERNAL_HEIGHT - y;
                break;
            }
        }
    }

    let center_avg = (heights[3] + heights[4] + heights[5] + heights[6]) as f32 / 4.0;
    let sides_avg = (heights[0] + heights[1] + heights[2] + heights[7] + heights[8] + heights[9]) as f32 / 6.0;

    if center_avg > sides_avg + 1.5 {
        ((center_avg - (sides_avg + 1.5)) / 6.0).min(1.0)
    } else {
        0.0
    }
}

/// 両端（x=0 と x=9）が同時に深さ2以上の縦穴になっている状態（Iミノ枯渇リスク）を検知
pub fn detect_dual_side_wells(board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT]) -> (bool, f32) {
    let mut heights = [0usize; BOARD_WIDTH];
    for x in 0..BOARD_WIDTH {
        for y in 0..INTERNAL_HEIGHT {
            if board[y][x].is_some() {
                heights[x] = INTERNAL_HEIGHT - y;
                break;
            }
        }
    }

    let depth_left = heights[1].saturating_sub(heights[0]);
    let depth_right = heights[8].saturating_sub(heights[9]);

    if depth_left >= 2 && depth_right >= 2 {
        (true, ((depth_left + depth_right) as f32 / 8.0).min(1.0))
    } else {
        (false, 0.0)
    }
}

/// Tスロットの位置（x座標）と穴の幅を評価 (2〜9列目、特に3〜8列目を高評価)
pub fn evaluate_t_slot_column_position(x: usize, notch_width: usize) -> f32 {
    if notch_width != 1 {
        return 0.1; // 幅1マス以外の穴はT-Spin後に崩れるため低評価
    }
    match x {
        2..=7 => 1.0,  // 最適: 3〜8列目の単一列穴
        1 | 8 => 0.85, // 準推奨: 2列目、9列目
        0 | 9 => 0.40, // 端スロットは減点
        _ => 0.5,
    }
}

/// 壁端（x=0 または x=9）における TST（T-Spin Triple）の屋根向きの物理的成立性を検証
pub fn validate_wall_tst_orientation(
    board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT],
    tst_x: usize,
    tst_y: usize,
) -> (bool, f32) {
    if tst_x == 0 {
        // 左壁TST: 屋根は必ず盤面内側(x=1)から伸びる内向きである必要がある
        let has_inner_roof = tst_y >= 2 && (board[tst_y - 1][1].is_some() || board[tst_y - 2][1].is_some());
        let has_inner_peg = tst_y + 1 < INTERNAL_HEIGHT && board[tst_y + 1][1].is_some();
        if has_inner_roof && has_inner_peg {
            (true, 1.0)
        } else {
            (false, 0.0) // 空中(x=-1)に屋根を要求する不正配置
        }
    } else if tst_x == BOARD_WIDTH - 1 {
        // 右壁TST: 屋根は必ず盤面内側(x=8)から伸びる内向きである必要がある
        let has_inner_roof = tst_y >= 2 && (board[tst_y - 1][BOARD_WIDTH - 2].is_some() || board[tst_y - 2][BOARD_WIDTH - 2].is_some());
        let has_inner_peg = tst_y + 1 < INTERNAL_HEIGHT && board[tst_y + 1][BOARD_WIDTH - 2].is_some();
        if has_inner_roof && has_inner_peg {
            (true, 1.0)
        } else {
            (false, 0.0)
        }
    } else {
        // 内側TSTは物理的に成立可能
        (true, 0.9)
    }
}

/// しゑひ式「階段のドネイト (Kaidan Setups)」パターンを検出
pub fn detect_kaidan_setup_patterns(board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT]) -> f32 {
    let mut kaidan_quality = 0.0f32;
    for cy in 1..(INTERNAL_HEIGHT - 1) {
        for cx in 1..(BOARD_WIDTH - 2) {
            // 階段状の段差 (高低差1マス)
            let step_up_right = cy + 1 < INTERNAL_HEIGHT && board[cy + 1][cx].is_some() && board[cy][cx + 1].is_some() && board[cy + 1][cx + 1].is_some();
            let step_up_left = cy + 1 < INTERNAL_HEIGHT && board[cy + 1][cx + 1].is_some() && board[cy][cx].is_some() && board[cy + 1][cx].is_some();

            if step_up_right || step_up_left {
                // ドネイトブロックによる屋根
                let has_overhang = if step_up_right {
                    board[cy.saturating_sub(1)][cx].is_none() && (cy >= 1 && board[cy - 1][cx + 1].is_some())
                } else {
                    board[cy.saturating_sub(1)][cx + 1].is_none() && (cy >= 1 && board[cy - 1][cx].is_some())
                };

                if has_overhang {
                    // 内側列(x=2..7)であれば高評価
                    let pos_bonus = if cx >= 2 && cx <= 6 { 1.0 } else { 0.8 };
                    kaidan_quality = kaidan_quality.max(0.90 * pos_bonus);
                }
            }
        }
    }
    kaidan_quality
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_line_clearing() {
        let mut game = Game::new();
        // 一番下の行をすべてIミノブロックで埋める
        let bottom_y = INTERNAL_HEIGHT - 1;
        for x in 0..BOARD_WIDTH {
            game.board[bottom_y][x] = Some(BlockType::I);
        }
        
        let cleared = game.clear_lines();
        assert_eq!(cleared, 1);
        
        // 消去後、一番下の行が空になっていることを確認
        for x in 0..BOARD_WIDTH {
            assert!(game.board[bottom_y][x].is_none());
        }
    }

    #[test]
    fn test_srs_kick_t_piece() {
        let mut game = Game::new();
        // Tミノを左壁際に密着させる
        game.current_piece = Piece::new(BlockType::T);
        game.current_piece.x = 0; // 左端
        game.current_piece.rotation = 0;
        
        // 左回転を試みる (0 -> 3)。
        // 回転すると左側がはみ出るため、SRSで右にキックされて回転が成功するはず。
        let success = game.try_rotate(RotationDirection::CounterClockwise);
        assert!(success);
        assert!(game.current_piece.x >= 0);
    }

    #[test]
    fn test_well_bonus() {
        use crate::config::game::{WELL_BASE_SCORE_EDGE, WELL_BASE_SCORE_MIDDLE, WELL_BASE_SCORE_TARGET};

        // --- 1. 1列目 (index 0) の穴 ---
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        for y in (INTERNAL_HEIGHT - 3)..INTERNAL_HEIGHT {
            board[y][1] = Some(BlockType::O);
        }
        // index 0 はエッジスコア (WELL_BASE_SCORE_EDGE)
        assert_eq!(get_well_bonus(&board), WELL_BASE_SCORE_EDGE);

        // 深さ4なら3倍
        board[INTERNAL_HEIGHT - 4][1] = Some(BlockType::O);
        assert_eq!(get_well_bonus(&board), WELL_BASE_SCORE_EDGE * 3);

        // ほかの列に穴をあけると0
        board[INTERNAL_HEIGHT - 2][1] = None;
        assert_eq!(get_well_bonus(&board), 0);
        // 元に戻す
        board[INTERNAL_HEIGHT - 2][1] = Some(BlockType::O);
        assert_eq!(get_well_bonus(&board), WELL_BASE_SCORE_EDGE * 3);

        // 2列以上に穴があれば0
        for y in (INTERNAL_HEIGHT - 3)..INTERNAL_HEIGHT {
            board[y][8] = Some(BlockType::O);
        }
        assert_eq!(get_well_bonus(&board), 0);

        // --- 2. 2列目 (index 1) の穴 ---
        let mut board2 = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        for y in (INTERNAL_HEIGHT - 3)..INTERNAL_HEIGHT {
            board2[y][0] = Some(BlockType::O);
            board2[y][2] = Some(BlockType::O);
        }
        // index 1 はミドルスコア (WELL_BASE_SCORE_MIDDLE)
        assert_eq!(get_well_bonus(&board2), WELL_BASE_SCORE_MIDDLE);

        // --- 3. 7列目 (index 6) の穴 ---
        let mut board3 = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        for y in (INTERNAL_HEIGHT - 3)..INTERNAL_HEIGHT {
            board3[y][5] = Some(BlockType::O);
            board3[y][7] = Some(BlockType::O);
        }
        // index 6 はターゲットスコア (WELL_BASE_SCORE_TARGET)
        assert_eq!(get_well_bonus(&board3), WELL_BASE_SCORE_TARGET);

        // 深さ4なら3倍
        board3[INTERNAL_HEIGHT - 4][5] = Some(BlockType::O);
        board3[INTERNAL_HEIGHT - 4][7] = Some(BlockType::O);
        assert_eq!(get_well_bonus(&board3), WELL_BASE_SCORE_TARGET * 3);
    }

    #[test]
    fn test_count_t_slots() {
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        assert_eq!(count_t_slots(&board), 0);

        let cy = INTERNAL_HEIGHT - 2;
        let cx = 2;
        // 左側を埋める
        board[cy][cx - 1] = Some(BlockType::I);
        // コーナー3つを埋める (右上、左下、右下)
        board[cy - 1][cx + 1] = Some(BlockType::I);
        board[cy + 1][cx - 1] = Some(BlockType::I);
        board[cy + 1][cx + 1] = Some(BlockType::I);

        let c = count_t_slots(&board);
        assert_eq!(c, 1, "Expected 1 slot but got {}", c);
    }

    #[test]
    fn test_t_spin_detection_full_and_mini() {
        let mut game = Game::new();
        game.board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        
        let cy = INTERNAL_HEIGHT - 2;
        let cx = 2;
        
        // 1. T-Spin Mini: rotation 2 (pointing down), 1 front corner (Bottom-Left) + 2 back corners (Top-Left, Top-Right)
        game.board[cy - 1][cx - 1] = Some(BlockType::I); // Top-Left (Back)
        game.board[cy - 1][cx + 1] = Some(BlockType::I); // Top-Right (Back)
        game.board[cy + 1][cx - 1] = Some(BlockType::I); // Bottom-Left (Front)
        game.board[cy + 1][cx + 1] = None;               // Bottom-Right (Front) - empty
        
        let mini_piece = Piece {
            block_type: BlockType::T,
            x: cx as i32,
            y: cy as i32,
            rotation: 2,
        };
        
        let mini_res = check_t_spin_type(&game.board, &mini_piece, true, false);
        assert_eq!(mini_res, TSpinResult::Mini(0));

        // 2. Full T-Spin: Both front corners filled (Bottom-Left, Bottom-Right) + 1 back corner (Top-Left)
        game.board[cy + 1][cx + 1] = Some(BlockType::I); // Bottom-Right (Front) - filled!
        let full_res = check_t_spin_type(&game.board, &mini_piece, true, false);
        assert_eq!(full_res, TSpinResult::Full(0));

        // 3. Non-T-Spin: was_rotate == false
        let no_rotate_res = check_t_spin_type(&game.board, &mini_piece, false, false);
        assert_eq!(no_rotate_res, TSpinResult::None);

        // 4. Wall drop without rotation
        let wall_game = Game::new();
        let wall_piece = Piece {
            block_type: BlockType::T,
            x: 0,
            y: 15,
            rotation: 1,
        };
        let wall_res = check_t_spin_type(&wall_game.board, &wall_piece, false, false);
        assert_eq!(wall_res, TSpinResult::None);
    }

    #[test]
    fn test_td_cannon_detection() {
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let bottom = INTERNAL_HEIGHT - 1;

        // 壁際(x=0)に深さ3マスの縦溝を構築 (y=bottom-3..=bottom-1 が空洞、y=bottom が床)
        for y in (bottom - 4)..=bottom {
            for x in 0..BOARD_WIDTH {
                board[y][x] = Some(BlockType::I);
            }
        }
        board[bottom - 3][0] = None;
        board[bottom - 2][0] = None;
        board[bottom - 1][0] = None;
        // TD砲のシェルフ屋根 (x=1, y=bottom-3) を残し、(x=1, y=bottom-4) を空にしてオーバーハングを作成
        board[bottom - 3][1] = Some(BlockType::J);
        board[bottom - 4][1] = None;

        let quality = evaluate_t_spin_terrain(&board);
        assert!(quality >= 0.90, "TD Cannon structure should yield >= 0.90 quality score, got {}", quality);
    }

    #[test]
    fn test_versus_garbage_cancellation_and_downstack() {
        let mut game = Game::new();
        game.pending_garbage = 4; // 4 lines incoming

        // 1 line clear (firepower 0) cancels 0 garbage
        let bottom = INTERNAL_HEIGHT - 1;
        for x in 0..BOARD_WIDTH {
            game.board[bottom][x] = Some(BlockType::I);
        }
        let cleared = game.clear_lines();
        assert_eq!(cleared, 1);
        assert_eq!(game.pending_garbage, 4);

        // Apply garbage
        game.apply_garbage();
        assert_eq!(game.pending_garbage, 0);
        // Bottom 4 lines should now have garbage
        for y in (INTERNAL_HEIGHT - 4)..INTERNAL_HEIGHT {
            let filled_count = game.board[y].iter().filter(|c| c.is_some()).count();
            assert_eq!(filled_count, BOARD_WIDTH - 1, "Garbage row should have exactly 9 blocks and 1 hole");
        }
    }

    #[test]
    fn test_shiwehi_donations_and_stsd() {
        // 1. Shiwehi S-Donate (階段のドネイト)
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let bottom = INTERNAL_HEIGHT - 1;
        for y in (bottom - 3)..=bottom {
            for x in 1..BOARD_WIDTH {
                board[y][x] = Some(BlockType::I);
            }
        }
        // Column 0 is the well (y = bottom-3..=bottom is empty at x=0)
        // S-Donate forms an overhang at (x=1, y=bottom-2) creating a 3-wide T-slot at columns 0, 1, 2
        board[bottom - 2][0] = None;
        board[bottom - 1][0] = None;
        board[bottom - 2][1] = Some(BlockType::S); // S-roof
        board[bottom - 2][2] = None;               // Slot opening

        let donation_quality = evaluate_t_spin_terrain(&board);
        assert!(donation_quality >= 0.80, "Shiwehi S-Donate should yield >= 0.80 quality, got {}", donation_quality);

        // 2. STSD (Super T-Spin Double)
        let mut stsd_board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        for y in (bottom - 4)..=bottom {
            for x in 0..BOARD_WIDTH {
                stsd_board[y][x] = Some(BlockType::I);
            }
        }
        stsd_board[bottom - 1][4] = None;
        stsd_board[bottom - 1][3] = None;
        stsd_board[bottom - 1][5] = None;
        stsd_board[bottom - 2][4] = None;
        stsd_board[bottom - 2][3] = Some(BlockType::L); // STSD double roof
        stsd_board[bottom - 3][3] = Some(BlockType::L);

        let stsd_quality = evaluate_t_spin_terrain(&stsd_board);
        assert!(stsd_quality >= 0.90, "STSD structure should yield >= 0.90 quality, got {}", stsd_quality);
    }

    #[test]
    fn test_center_convexity_calculation() {
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let bottom = INTERNAL_HEIGHT - 1;

        // 1. Flat terrain -> 0.0 convexity
        for x in 0..BOARD_WIDTH {
            board[bottom][x] = Some(BlockType::I);
            board[bottom - 1][x] = Some(BlockType::I);
        }
        assert_eq!(calculate_center_convexity(&board), 0.0, "Flat terrain should have 0.0 convexity");

        // 2. Central mountain (columns 3, 4, 5, 6 high, 0..2 and 7..9 low)
        for y in (bottom - 5)..=(bottom - 2) {
            board[y][3] = Some(BlockType::I);
            board[y][4] = Some(BlockType::I);
            board[y][5] = Some(BlockType::I);
            board[y][6] = Some(BlockType::I);
        }
        let conv = calculate_center_convexity(&board);
        assert!(conv > 0.3, "Central mountain should produce high convexity penalty, got {}", conv);
    }

    #[test]
    fn test_dual_side_well_detection() {
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let bottom = INTERNAL_HEIGHT - 1;

        // Columns 1..8 are 4 blocks high, columns 0 and 9 are 0 blocks high (both sides open!)
        for y in (bottom - 3)..=bottom {
            for x in 1..=8 {
                board[y][x] = Some(BlockType::I);
            }
        }
        let (is_dual, sev) = detect_dual_side_wells(&board);
        assert!(is_dual, "Simultaneous deep wells on left (x=0) and right (x=9) must be flagged as dual side wells");
        assert!(sev > 0.5, "Dual side well severity should be > 0.5, got {}", sev);

        // Fill column 0 -> now only 1 well on column 9 (Single well is valid)
        for y in (bottom - 3)..=bottom {
            board[y][0] = Some(BlockType::I);
        }
        let (is_dual_single, _) = detect_dual_side_wells(&board);
        assert!(!is_dual_single, "Single well on column 9 must NOT be flagged as dual side well");
    }

    #[test]
    fn test_t_slot_column_position_rating() {
        // 3〜8列目 (x=2..7): 最適 (1.0)
        assert_eq!(evaluate_t_slot_column_position(2, 1), 1.0);
        assert_eq!(evaluate_t_slot_column_position(5, 1), 1.0);
        assert_eq!(evaluate_t_slot_column_position(7, 1), 1.0);

        // 2列目 (x=1), 9列目 (x=8): 準推奨 (0.85)
        assert_eq!(evaluate_t_slot_column_position(1, 1), 0.85);
        assert_eq!(evaluate_t_slot_column_position(8, 1), 0.85);

        // 1列目 (x=0), 10列目 (x=9): 端スロット減点 (0.40)
        assert_eq!(evaluate_t_slot_column_position(0, 1), 0.40);
        assert_eq!(evaluate_t_slot_column_position(9, 1), 0.40);
    }

    #[test]
    fn test_wall_tst_orientation_valid_and_invalid() {
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let bottom = INTERNAL_HEIGHT - 1;

        // 1. Left wall TST with inner roof at x=1 (VALID)
        let tst_y = bottom - 2;
        board[tst_y - 1][1] = Some(BlockType::L); // Inner roof
        board[tst_y + 1][1] = Some(BlockType::L); // Lower peg
        let (valid_left, q_left) = validate_wall_tst_orientation(&board, 0, tst_y);
        assert!(valid_left, "Left wall TST with inner roof must be valid");
        assert_eq!(q_left, 1.0);

        // 2. Left wall TST with no inner roof (requires impossible floating roof outside board at x=-1) (INVALID)
        let invalid_board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let (valid_invalid, _) = validate_wall_tst_orientation(&invalid_board, 0, tst_y);
        assert!(!valid_invalid, "Left wall TST without inner roof must be INVALID");
    }

    #[test]
    fn test_kaidan_setup_detection() {
        let mut board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        let bottom = INTERNAL_HEIGHT - 1;

        // Step up at column 3, 4 with roof at (x=4, y=bottom-2)
        board[bottom][2] = Some(BlockType::I);
        board[bottom][3] = Some(BlockType::I);
        board[bottom][4] = Some(BlockType::I);
        board[bottom - 1][4] = Some(BlockType::I);
        board[bottom - 2][4] = Some(BlockType::S); // S-階段

        let kaidan_q = detect_kaidan_setup_patterns(&board);
        assert!(kaidan_q >= 0.70, "Kaidan setup should be detected with quality >= 0.70, got {}", kaidan_q);
    }
}
