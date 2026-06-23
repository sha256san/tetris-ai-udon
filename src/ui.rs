use std::io::{stdout, Write, Stdout};
use crossterm::{
    execute, queue, cursor, terminal,
    style::{Color, Print, ResetColor, SetBackgroundColor, SetForegroundColor},
};
use crate::tetris::{Game, Piece, BlockType, BOARD_WIDTH, BOARD_HEIGHT, INTERNAL_HEIGHT};
use crate::ai::AiModel;

pub const UI_X_OFFSET: u16 = 15;
pub const UI_Y_OFFSET: u16 = 2;

fn get_block_color(bt: BlockType) -> Color {
    match bt {
        BlockType::I => Color::Rgb { r: 0, g: 220, b: 220 },   // シアン
        BlockType::O => Color::Rgb { r: 220, g: 220, b: 0 },   // イエロー
        BlockType::T => Color::Rgb { r: 180, g: 0, b: 220 },   // マゼンタ
        BlockType::S => Color::Rgb { r: 0, g: 220, b: 0 },     // グリーン
        BlockType::Z => Color::Rgb { r: 220, g: 0, b: 0 },     // レッド
        BlockType::J => Color::Rgb { r: 0, g: 80, b: 220 },    // ブルー
        BlockType::L => Color::Rgb { r: 220, g: 120, b: 0 },   // オレンジ
    }
}

// ターミナルの初期化
pub fn init_terminal() -> std::io::Result<()> {
    terminal::enable_raw_mode()?;
    let mut out = stdout();
    execute!(out, terminal::EnterAlternateScreen, cursor::Hide, terminal::Clear(terminal::ClearType::All))?;
    Ok(())
}

// ターミナルの復元
pub fn restore_terminal() -> std::io::Result<()> {
    let mut out = stdout();
    execute!(out, cursor::Show, terminal::LeaveAlternateScreen)?;
    terminal::disable_raw_mode()?;
    Ok(())
}

// ゴーストミノ（落下予測地点）のY座標を取得
pub fn get_ghost_y(game: &Game) -> i32 {
    let mut ghost = game.current_piece.clone();
    while game.is_valid_position(&Piece {
        block_type: ghost.block_type,
        x: ghost.x,
        y: ghost.y + 1,
        rotation: ghost.rotation,
    }) {
        ghost.y += 1;
    }
    ghost.y
}

// ゲーム画面全体を描画
pub fn draw_game(
    game: &Game,
    model: &AiModel,
    mode_name: &str,
    rl_stats: Option<(usize, f32, f32)>, // (episodes, avg_lines, epsilon)
) -> std::io::Result<()> {
    let mut out = stdout();

    // 1. タイトルとモード表示
    queue!(
        out,
        cursor::MoveTo(UI_X_OFFSET, UI_Y_OFFSET - 2),
        SetForegroundColor(Color::Cyan),
        Print("=== ANTIGRAVITY TETRIS AI ==="),
        cursor::MoveTo(UI_X_OFFSET, UI_Y_OFFSET - 1),
        SetForegroundColor(Color::White),
        Print(format!("Mode: {}  |  Score: {}  |  Lines: {}", mode_name, game.score, game.lines_cleared)),
        ResetColor
    )?;

    // 2. 左側：HOLD枠の描画
    draw_hold_box(&mut out, game)?;

    // 3. 左側：AIの重みパラメータの描画
    draw_ai_weights(&mut out, model)?;

    // 4. 中央：テトリス盤面の描画
    draw_board(&mut out, game)?;

    // 5. 右側：NEXT枠の描画
    draw_next_box(&mut out, game)?;

    // 6. 右側：学習の統計情報（もしあれば）
    if let Some((ep, avg_lines, eps)) = rl_stats {
        queue!(
            out,
            cursor::MoveTo(UI_X_OFFSET + 26, UI_Y_OFFSET + 12),
            SetForegroundColor(Color::Magenta),
            Print("== RL STATS =="),
            cursor::MoveTo(UI_X_OFFSET + 26, UI_Y_OFFSET + 13),
            SetForegroundColor(Color::White),
            Print(format!("Episodes: {}", ep)),
            cursor::MoveTo(UI_X_OFFSET + 26, UI_Y_OFFSET + 14),
            Print(format!("Avg Lines: {:.2}", avg_lines)),
            cursor::MoveTo(UI_X_OFFSET + 26, UI_Y_OFFSET + 15),
            Print(format!("Epsilon: {:.4}", eps)),
            ResetColor
        )?;
    }

    out.flush()?;
    Ok(())
}

fn draw_board(out: &mut Stdout, game: &Game) -> std::io::Result<()> {
    let border_color = Color::Rgb { r: 100, g: 100, b: 120 };
    
    // 上枠
    queue!(out, cursor::MoveTo(UI_X_OFFSET - 1, UI_Y_OFFSET))?;
    queue!(out, SetForegroundColor(border_color), Print("┌"), ResetColor)?;
    for _ in 0..BOARD_WIDTH {
        queue!(out, SetForegroundColor(border_color), Print("──"), ResetColor)?;
    }
    queue!(out, SetForegroundColor(border_color), Print("┐"), ResetColor)?;

    let ghost_y = get_ghost_y(game);
    let piece_cells = game.current_piece.get_cells();
    
    // ゴーストミノのセル
    let mut ghost_cells = [(0, 0); 4];
    let offsets = crate::tetris::get_piece_offsets(game.current_piece.block_type, game.current_piece.rotation);
    for i in 0..4 {
        ghost_cells[i] = (game.current_piece.x + offsets[i].0, ghost_y + offsets[i].1);
    }

    // 盤面ブロック (INTERNAL_HEIGHTの上部バッファをカットし、下部BOARD_HEIGHT=20行のみ描画)
    for y_idx in 0..BOARD_HEIGHT {
        let internal_y = y_idx + (INTERNAL_HEIGHT - BOARD_HEIGHT);
        queue!(out, cursor::MoveTo(UI_X_OFFSET - 1, UI_Y_OFFSET + 1 + y_idx as u16))?;
        queue!(out, SetForegroundColor(border_color), Print("│"), ResetColor)?;

        for x in 0..BOARD_WIDTH {
            let cx = x as i32;
            let cy = internal_y as i32;

            if let Some(bt) = game.board[internal_y][x] {
                // 固定ブロック
                queue!(
                    out,
                    SetBackgroundColor(get_block_color(bt)),
                    Print("  "),
                    ResetColor
                )?;
            } else if piece_cells.contains(&(cx, cy)) {
                // 現在操作中のブロック
                queue!(
                    out,
                    SetBackgroundColor(get_block_color(game.current_piece.block_type)),
                    Print("  "),
                    ResetColor
                )?;
            } else if ghost_cells.contains(&(cx, cy)) && !game.game_over {
                // ゴースト（落下予測）ブロック
                queue!(
                    out,
                    SetForegroundColor(Color::DarkGrey),
                    Print("░░"),
                    ResetColor
                )?;
            } else {
                // 空きマス
                // グリッドのドットを入れてプレミアム感を演出
                queue!(
                    out,
                    SetForegroundColor(Color::Rgb { r: 40, g: 40, b: 50 }),
                    Print(" ∙"),
                    ResetColor
                )?;
            }
        }

        queue!(out, SetForegroundColor(border_color), Print("│"), ResetColor)?;
    }

    // 下枠
    queue!(out, cursor::MoveTo(UI_X_OFFSET - 1, UI_Y_OFFSET + 1 + BOARD_HEIGHT as u16))?;
    queue!(out, SetForegroundColor(border_color), Print("└"), ResetColor)?;
    for _ in 0..BOARD_WIDTH {
        queue!(out, SetForegroundColor(border_color), Print("──"), ResetColor)?;
    }
    queue!(out, SetForegroundColor(border_color), Print("┘"), ResetColor)?;

    Ok(())
}

fn draw_hold_box(out: &mut Stdout, game: &Game) -> std::io::Result<()> {
    let x_pos = UI_X_OFFSET - 12;
    let y_pos = UI_Y_OFFSET;
    let border_color = Color::Rgb { r: 100, g: 100, b: 120 };

    // 枠描画
    queue!(out, cursor::MoveTo(x_pos, y_pos), SetForegroundColor(border_color), Print("┌──HOLD──┐"), ResetColor)?;
    for i in 1..4 {
        queue!(out, cursor::MoveTo(x_pos, y_pos + i), SetForegroundColor(border_color), Print("│        │"), ResetColor)?;
    }
    queue!(out, cursor::MoveTo(x_pos, y_pos + 4), SetForegroundColor(border_color), Print("└────────┘"), ResetColor)?;

    // ホールドミノの描画
    if let Some(bt) = game.hold_piece {
        // ミノ形状のローカル座標(状態0)を取得
        let offsets = crate::tetris::get_piece_offsets(bt, 0);
        let color = get_block_color(bt);

        // 中央揃え用の微調整
        let (ox, oy) = match bt {
            BlockType::I => (2, 2),
            BlockType::O => (3, 2),
            _ => (3, 2),
        };

        for &(dx, dy) in &offsets {
            let px = x_pos + 1 + ((dx + ox) as u16) * 2;
            let py = y_pos + 1 + (dy + oy) as u16;
            if py < y_pos + 4 {
                queue!(
                    out,
                    cursor::MoveTo(px, py),
                    SetBackgroundColor(color),
                    Print("  "),
                    ResetColor
                )?;
            }
        }
    }

    Ok(())
}

fn draw_next_box(out: &mut Stdout, game: &Game) -> std::io::Result<()> {
    let x_pos = UI_X_OFFSET + 24;
    let y_pos = UI_Y_OFFSET;
    let border_color = Color::Rgb { r: 100, g: 100, b: 120 };

    // Nextを3つ描画
    queue!(out, cursor::MoveTo(x_pos, y_pos), SetForegroundColor(border_color), Print("┌──NEXT──┐"), ResetColor)?;
    for i in 1..10 {
        queue!(out, cursor::MoveTo(x_pos, y_pos + i), SetForegroundColor(border_color), Print("│        │"), ResetColor)?;
    }
    queue!(out, cursor::MoveTo(x_pos, y_pos + 10), SetForegroundColor(border_color), Print("└────────┘"), ResetColor)?;

    let next_pieces = game.bag.peek_next(3);
    for (idx, &bt) in next_pieces.iter().enumerate() {
        let offsets = crate::tetris::get_piece_offsets(bt, 0);
        let color = get_block_color(bt);

        let (ox, oy) = match bt {
            BlockType::I => (2, 1),
            BlockType::O => (3, 1),
            _ => (3, 1),
        };

        let piece_y_offset = y_pos + 1 + (idx as u16) * 3;

        for &(dx, dy) in &offsets {
            let px = x_pos + 1 + ((dx + ox) as u16) * 2;
            let py = piece_y_offset + (dy + oy) as u16;
            queue!(
                out,
                cursor::MoveTo(px, py),
                SetBackgroundColor(color),
                Print("  "),
                ResetColor
            )?;
        }
    }

    Ok(())
}

fn draw_ai_weights(out: &mut Stdout, model: &AiModel) -> std::io::Result<()> {
    let x_pos = UI_X_OFFSET - 12;
    let y_pos = UI_Y_OFFSET + 6;
    let border_color = Color::Rgb { r: 100, g: 100, b: 120 };

    queue!(
        out,
        cursor::MoveTo(x_pos, y_pos),
        SetForegroundColor(border_color),
        Print("┌──AI WEIGHTS──┐"),
        ResetColor
    )?;
    
    let labels = [
        "Max H",
        "Avg H",
        "Bumpy",
        "Holes",
        "Abv H",
        "Wells",
        "Clr13",
        "Tetrs",
    ];

    for i in 0..8 {
        queue!(
            out,
            cursor::MoveTo(x_pos, y_pos + 1 + i as u16),
            SetForegroundColor(border_color),
            Print("│"),
            SetForegroundColor(Color::Yellow),
            Print(format!(" {:<5}:{:>6.2}", labels[i], model.weights[i])),
            SetForegroundColor(border_color),
            Print(" │"),
            ResetColor
        )?;
    }

    queue!(
        out,
        cursor::MoveTo(x_pos, y_pos + 9),
        SetForegroundColor(border_color),
        Print("└──────────────┘"),
        ResetColor
    )?;

    Ok(())
}
