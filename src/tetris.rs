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
        for &(dx, dy) in &kick_offsets {
            let mut test_piece = next_piece.clone();
            test_piece.x += dx;
            test_piece.y += dy;
            if self.is_valid_position(&test_piece) {
                self.current_piece = test_piece;
                self.last_action_was_rotate = true;
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

        for &(cx, cy) in &self.current_piece.get_cells() {
            if cx >= 0 && cx < BOARD_WIDTH as i32 && cy >= 0 && cy < INTERNAL_HEIGHT as i32 {
                self.board[cy as usize][cx as usize] = Some(self.current_piece.block_type);
            }
        }

        // T-spinの判定 (ライン消去の前に行う)
        let mut is_t_spin = false;
        if is_t_piece && was_rotate {
            let cx = self.current_piece.x;
            let cy = self.current_piece.y;
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
                } else if self.board[y as usize][x as usize].is_some() {
                    filled_corners += 1;
                }
            }
            if filled_corners >= 3 {
                is_t_spin = true;
            }
        }

        // ライン消去
        let cleared = self.clear_lines();
        self.lines_cleared += cleared as u32;

        if is_t_spin {
            let (score_add, t_spin_name) = match cleared {
                0 => (crate::config::game::TSPIN_0_SCORE, "T-Spin"),
                1 => (crate::config::game::TSPIN_1_SCORE, "T-Spin Single"),
                2 => (crate::config::game::TSPIN_2_SCORE, "T-Spin Double"),
                3 => (crate::config::game::TSPIN_3_SCORE, "T-Spin Triple"),
                _ => (0, "T-Spin"),
            };
            self.score += score_add;
            self.last_t_spin = Some(t_spin_name.to_string());
        } else {
            if cleared < crate::config::game::LINE_CLEAR_SCORES.len() {
                self.score += crate::config::game::LINE_CLEAR_SCORES[cleared];
            }
            self.last_t_spin = None;
        }

        // Damage Calculation
        let mut firepower = 0;
        let mut is_btb_eligible = false;

        if is_t_spin {
            match cleared {
                1 => { firepower = 1; is_btb_eligible = true; } // TSS
                2 => { firepower = 4; is_btb_eligible = true; } // TSD
                3 => { firepower = 6; is_btb_eligible = true; } // TST
                _ => {}
            }
        } else {
            match cleared {
                1 => { firepower = 0; }
                2 => { firepower = 1; }
                3 => { firepower = 2; }
                4 => { firepower = 4; is_btb_eligible = true; } // Tetris
                _ => {}
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

/// 盤面上の T-spin slot (T-slot) の個数をカウントする
pub fn count_t_slots(board: &[[Option<BlockType>; BOARD_WIDTH]; INTERNAL_HEIGHT]) -> usize {
    let mut count = 0;
    for cy in 1..(INTERNAL_HEIGHT - 1) {
        for cx in 1..(BOARD_WIDTH - 1) {
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
            for &(x, y) in &corners {
                if x < 0 || x >= BOARD_WIDTH as i32 || y < 0 || y >= INTERNAL_HEIGHT as i32 {
                    filled_corners += 1;
                } else if board[y as usize][x as usize].is_some() {
                    filled_corners += 1;
                }
            }

            if filled_corners < 3 {
                continue;
            }

            // 4つのTの向きについて判定 (0: 上, 1: 右, 2: 下, 3: 左)
            // それぞれの向きで、Tミノが入る3つのセルが空で、かつ反対側（Tミノの底辺中央の隣）が埋まっているかチェック
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
                let is_blocked = if bx < 0 || bx >= BOARD_WIDTH as i32 || by < 0 || by >= INTERNAL_HEIGHT as i32 {
                    true
                } else {
                    board[by as usize][bx as usize].is_some()
                };

                if is_blocked {
                    count += 1;
                    break; // この中心セルに少なくとも1つの向きでT-slotが形成されている
                }
            }
        }
    }
    count
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
        // 左側を埋める (8, 0 に相当)
        board[cy][cx - 1] = Some(BlockType::I);
        // コーナー3つを埋める (右上、左下、右下)
        board[cy - 1][cx + 1] = Some(BlockType::I);
        board[cy + 1][cx - 1] = Some(BlockType::I);
        board[cy + 1][cx + 1] = Some(BlockType::I);

        assert_eq!(count_t_slots(&board), 1);
    }

    #[test]
    fn test_t_spin_detection() {
        let mut game = Game::new();
        game.board = [[None; BOARD_WIDTH]; INTERNAL_HEIGHT];
        
        let cy = INTERNAL_HEIGHT - 2;
        let cx = 2;
        
        // 3つのコーナーを埋める
        game.board[cy - 1][cx - 1] = Some(BlockType::I); // 左上
        game.board[cy - 1][cx + 1] = Some(BlockType::I); // 右上
        game.board[cy + 1][cx - 1] = Some(BlockType::I); // 左下
        
        game.current_piece = Piece {
            block_type: BlockType::T,
            x: cx as i32,
            y: cy as i32,
            rotation: 2,
        };
        
        game.last_action_was_rotate = true;
        game.lock_piece();
        
        assert!(game.last_t_spin.is_some());
    }
}
