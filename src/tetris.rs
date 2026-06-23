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
                return true;
            }
        }
        false
    }

    // SRSのキックオフセットテーブル (dx, dy) の取得。Y軸は下方向が正。
    fn get_kick_offsets(&self, block_type: BlockType, from_rot: usize, to_rot: usize) -> [(i32, i32); 5] {
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
        
        // ホールド直後に衝突している場合は即座にゲームオーバー
        if !self.is_valid_position(&self.current_piece) {
            self.game_over = true;
        }
        true
    }

    // ミノを固定し、ライン消去とネクストミノのスポーンを行う
    pub fn lock_piece(&mut self) {
        for &(cx, cy) in &self.current_piece.get_cells() {
            if cx >= 0 && cx < BOARD_WIDTH as i32 && cy >= 0 && cy < INTERNAL_HEIGHT as i32 {
                self.board[cy as usize][cx as usize] = Some(self.current_piece.block_type);
            }
        }

        // ライン消去
        let cleared = self.clear_lines();
        self.lines_cleared += cleared as u32;
        self.score += match cleared {
            1 => 100,
            2 => 300,
            3 => 500,
            4 => 800,
            _ => 0,
        };

        // 次のミノをスポーン
        let next_type = self.bag.pop();
        self.current_piece = Piece::new(next_type);
        self.hold_locked = false;

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
}
