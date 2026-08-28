use crate::tetris::{Game, BlockType, Piece, BOARD_WIDTH, INTERNAL_HEIGHT};
use serde::{Serialize, Deserialize};
use std::collections::VecDeque;
use std::fs::{self, File};
use std::io::Write;
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BoardSnapshot {
    pub turn: usize,
    pub relative_turn: i32, // -5, -4, -3, -2, -1, 0 (T-Spin), +1, +2, +3, +4, +5
    pub current_piece: Option<PieceInfo>,
    pub hold_piece: Option<String>,
    pub next_pieces: Vec<String>,
    pub lines_cleared_this_turn: u32,
    pub total_lines_cleared: u32,
    pub t_spin_type: Option<String>,
    pub score: u32,
    pub btb: bool,
    pub pending_garbage: u32,
    pub board_ascii: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PieceInfo {
    pub block_type: String,
    pub x: i32,
    pub y: i32,
    pub rotation: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TSpinEventRecord {
    pub event_id: usize,
    pub timestamp: String,
    pub t_spin_type: String,
    pub trigger_turn: usize,
    pub history_before_5: Vec<BoardSnapshot>,
    pub trigger_snapshot: BoardSnapshot,
    pub history_after_5: Vec<BoardSnapshot>,
}

#[derive(Debug)]
struct PendingTSpinEvent {
    event_id: usize,
    t_spin_type: String,
    trigger_turn: usize,
    history_before_5: Vec<BoardSnapshot>,
    trigger_snapshot: BoardSnapshot,
    history_after_5: Vec<BoardSnapshot>,
    remaining_after: usize,
}

pub struct TSpinRecorder {
    event_counter: usize,
    history_buffer: VecDeque<BoardSnapshot>, // 最大5件保持
    pending_events: Vec<PendingTSpinEvent>,
    output_dir: String,
}

impl TSpinRecorder {
    pub fn new() -> Self {
        let output_dir = "tspin_records".to_string();
        let _ = fs::create_dir_all(&output_dir);
        Self {
            event_counter: 0,
            history_buffer: VecDeque::with_capacity(5),
            pending_events: Vec::new(),
            output_dir,
        }
    }

    /// 各ターンの着地・固定直後に呼び出し
    pub fn record_turn(
        &mut self,
        turn: usize,
        game_before_spawn: &Game,
        placed_piece: &Piece,
        cleared_lines: u32,
        t_spin: Option<String>,
    ) {
        let t_spin_type_str = t_spin.clone();

        let snapshot = Self::create_snapshot(
            turn,
            0,
            game_before_spawn,
            Some(placed_piece),
            cleared_lines,
            t_spin.clone(),
        );

        // 1. T-Spin発生時の新規イベント登録
        if let Some(ref t_type) = t_spin {
            self.event_counter += 1;
            let mut before_snapshots: Vec<BoardSnapshot> = self.history_buffer.iter().cloned().collect();
            let count_before = before_snapshots.len();
            for (idx, snap) in before_snapshots.iter_mut().enumerate() {
                snap.relative_turn = -((count_before - idx) as i32);
            }

            let mut trigger_snap = snapshot.clone();
            trigger_snap.relative_turn = 0;

            self.pending_events.push(PendingTSpinEvent {
                event_id: self.event_counter,
                t_spin_type: t_type.clone(),
                trigger_turn: turn,
                history_before_5: before_snapshots,
                trigger_snapshot: trigger_snap,
                history_after_5: Vec::with_capacity(5),
                remaining_after: 5,
            });
        }

        // 2. 既存の保留中イベントに「直後5ターン」のスナップショットを追記
        let mut completed_indices = Vec::new();
        for (i, pending) in self.pending_events.iter_mut().enumerate() {
            // トリガーターンそのものは after に含めない
            if turn > pending.trigger_turn {
                let mut after_snap = snapshot.clone();
                let after_idx = 6 - pending.remaining_after; // 1, 2, 3, 4, 5
                after_snap.relative_turn = after_idx as i32;
                pending.history_after_5.push(after_snap);
                pending.remaining_after -= 1;

                if pending.remaining_after == 0 {
                    completed_indices.push(i);
                }
            }
        }

        // 3. 完了したイベントをファイルへ保存
        for &idx in completed_indices.iter().rev() {
            let pending = self.pending_events.remove(idx);
            self.save_event(&pending);
        }

        // 4. 履歴バッファの更新（直近5手）
        if self.history_buffer.len() >= 5 {
            self.history_buffer.pop_front();
        }
        let mut hist_snap = snapshot;
        hist_snap.t_spin_type = t_spin_type_str;
        self.history_buffer.push_back(hist_snap);
    }

    /// ゲーム終了時に未完のイベントがあれば保存
    pub fn flush_remaining(&mut self) {
        while let Some(pending) = self.pending_events.pop() {
            self.save_event(&pending);
        }
    }

    fn create_snapshot(
        turn: usize,
        relative_turn: i32,
        game: &Game,
        piece: Option<&Piece>,
        cleared_lines: u32,
        t_spin_type: Option<String>,
    ) -> BoardSnapshot {
        let piece_info = piece.map(|p| PieceInfo {
            block_type: format!("{:?}", p.block_type),
            x: p.x,
            y: p.y,
            rotation: p.rotation,
        });

        let hold_piece = game.hold_piece.map(|h| format!("{:?}", h));
        let next_pieces = game.bag.peek_next(5).iter().map(|b| format!("{:?}", b)).collect();

        // 盤面をASCII表現に変換（上部4行の不可視領域を除く20行）
        let mut board_ascii = Vec::new();
        for y in 4..INTERNAL_HEIGHT {
            let mut row_str = String::with_capacity(BOARD_WIDTH * 2 + 2);
            row_str.push('|');
            for x in 0..BOARD_WIDTH {
                if let Some(bt) = game.board[y][x] {
                    let ch = match bt {
                        BlockType::I => "I",
                        BlockType::O => "O",
                        BlockType::T => "T",
                        BlockType::S => "S",
                        BlockType::Z => "Z",
                        BlockType::J => "J",
                        BlockType::L => "L",
                    };
                    row_str.push_str(ch);
                    row_str.push(' ');
                } else {
                    row_str.push_str(". ");
                }
            }
            row_str.push('|');
            board_ascii.push(row_str);
        }

        BoardSnapshot {
            turn,
            relative_turn,
            current_piece: piece_info,
            hold_piece,
            next_pieces,
            lines_cleared_this_turn: cleared_lines,
            total_lines_cleared: game.lines_cleared,
            t_spin_type,
            score: game.score,
            btb: game.btb,
            pending_garbage: game.pending_garbage,
            board_ascii,
        }
    }

    fn save_event(&self, pending: &PendingTSpinEvent) {
        let ts = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let now = format!("ts_{}", ts);
        let safe_type = pending.t_spin_type.replace(' ', "_");
        let json_filename = format!(
            "{}/tspin_event_{:03}_{}_{}.json",
            self.output_dir, pending.event_id, safe_type, now
        );
        let txt_filename = format!(
            "{}/tspin_event_{:03}_{}_{}.txt",
            self.output_dir, pending.event_id, safe_type, now
        );

        let record = TSpinEventRecord {
            event_id: pending.event_id,
            timestamp: now.clone(),
            t_spin_type: pending.t_spin_type.clone(),
            trigger_turn: pending.trigger_turn,
            history_before_5: pending.history_before_5.clone(),
            trigger_snapshot: pending.trigger_snapshot.clone(),
            history_after_5: pending.history_after_5.clone(),
        };

        // 1. JSON 形式で保存
        if let Ok(file) = File::create(&json_filename) {
            let writer = std::io::BufWriter::new(file);
            let _ = serde_json::to_writer_pretty(writer, &record);
        }

        // 2. 視覚的なテキスト形式（ASCIIアート）で保存
        if let Ok(mut file) = File::create(&txt_filename) {
            let _ = writeln!(file, "================================================================================");
            let _ = writeln!(file, "  T-SPIN EVENT RECORD #{:03} : {} (Turn {})", pending.event_id, pending.t_spin_type, pending.trigger_turn);
            let _ = writeln!(file, "  Recorded at: {}", now);
            let _ = writeln!(file, "================================================================================\n");

            let mut all_steps = Vec::new();
            for snap in &pending.history_before_5 {
                all_steps.push(snap);
            }
            all_steps.push(&pending.trigger_snapshot);
            for snap in &pending.history_after_5 {
                all_steps.push(snap);
            }

            for snap in all_steps {
                let header = if snap.relative_turn == 0 {
                    format!("★ [Turn {} | T-SPIN TRIGGER: {}] ★", snap.turn, snap.t_spin_type.as_deref().unwrap_or("T-Spin"))
                } else if snap.relative_turn < 0 {
                    format!("▼ [Turn {} | {} turns BEFORE T-Spin]", snap.turn, snap.relative_turn.abs())
                } else {
                    format!("▲ [Turn {} | +{} turns AFTER T-Spin]", snap.turn, snap.relative_turn)
                };

                let _ = writeln!(file, "--------------------------------------------------------------------------------");
                let _ = writeln!(file, "{}", header);
                let _ = writeln!(file, "Piece: {:?} | Hold: {:?} | Next: {:?} | Score: {} | Lines: {} | BTB: {}",
                    snap.current_piece, snap.hold_piece, snap.next_pieces, snap.score, snap.total_lines_cleared, snap.btb);
                let _ = writeln!(file, "--------------------------------------------------------------------------------");
                for row in &snap.board_ascii {
                    let _ = writeln!(file, "  {}", row);
                }
                let _ = writeln!(file, "  +--------------------+\n");
            }
        }

        println!(" [T-Spin Recorder] Saved Event #{:03} ({}) -> {} & .txt",
            pending.event_id, pending.t_spin_type, json_filename);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tspin_recorder_5_before_and_5_after() {
        let mut recorder = TSpinRecorder::new();
        let game = Game::new();
        let piece = Piece::new(BlockType::I);

        // 1. Record 5 normal turns (turns 1..=5)
        for turn in 1..=5 {
            recorder.record_turn(turn, &game, &piece, 0, None);
        }

        // 2. Trigger T-Spin on turn 6
        let t_piece = Piece::new(BlockType::T);
        recorder.record_turn(6, &game, &t_piece, 2, Some("T-Spin Double".to_string()));

        assert_eq!(recorder.pending_events.len(), 1);
        assert_eq!(recorder.pending_events[0].history_before_5.len(), 5);
        assert_eq!(recorder.pending_events[0].remaining_after, 5);

        // 3. Record 5 subsequent turns (turns 7..=11)
        for turn in 7..=11 {
            recorder.record_turn(turn, &game, &piece, 0, None);
        }

        // After turn 11, the event should be finalized and saved
        assert_eq!(recorder.pending_events.len(), 0);
    }
}
