mod tetris;
mod ai;
mod imitation;
mod rl;
mod ui;

use std::fs::File;
use std::io::{stdout, Write};
use std::time::{Duration, Instant};
use crossterm::{
    execute, queue, cursor, terminal,
    event::{self, Event, KeyCode},
    style::{Color, Print, ResetColor, SetForegroundColor, SetBackgroundColor},
};
use tetris::{Game, RotationDirection};
use ai::{AiModel, enumerate_all_moves};
use imitation::{ExpertStep, save_log, load_log, train_one_epoch};

const MODEL_PATH: &str = "model.json";
const DATASET_PATH: &str = "dataset.json";

fn load_model_or_default() -> AiModel {
    if let Ok(file) = File::open(MODEL_PATH) {
        let reader = std::io::BufReader::new(file);
        if let Ok(model) = serde_json::from_reader::<_, AiModel>(reader) {
            if model.weights.len() == 8 {
                return model;
            }
        }
    }
    AiModel::new_default()
}

fn save_model(model: &AiModel) -> std::io::Result<()> {
    let file = File::create(MODEL_PATH)?;
    let writer = std::io::BufWriter::new(file);
    serde_json::to_writer_pretty(writer, model)?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    // ターミナルの初期化
    ui::init_terminal()?;

    let mut model = load_model_or_default();

    loop {
        let selection = show_menu(&model)?;
        match selection {
            1 => run_play_mode(&mut model)?,
            2 => run_ai_mode(&model)?,
            3 => run_imitation_mode(&mut model)?,
            4 => run_rl_mode(&mut model)?,
            _ => break, // Exit
        }
    }

    // ターミナルの復元
    ui::restore_terminal()?;
    Ok(())
}

// メインメニューの表示と選択
fn show_menu(model: &AiModel) -> std::io::Result<u8> {
    let mut out = stdout();
    execute!(out, terminal::Clear(terminal::ClearType::All), cursor::MoveTo(0, 0))?;

    let menu_x = 5;
    let menu_y = 3;

    queue!(
        out,
        cursor::MoveTo(menu_x, menu_y),
        SetForegroundColor(Color::Cyan),
        Print("========================================="),
        cursor::MoveTo(menu_x, menu_y + 1),
        Print("       ANTIGRAVITY TETRIS AI SYSTEM      "),
        cursor::MoveTo(menu_x, menu_y + 2),
        Print("========================================="),
        ResetColor,
        cursor::MoveTo(menu_x, menu_y + 4),
        SetForegroundColor(Color::White),
        Print("Select Mode:"),
        cursor::MoveTo(menu_x + 2, menu_y + 6),
        SetForegroundColor(Color::Green),
        Print("[1] Human Play Mode (Collect Expert Data)"),
        cursor::MoveTo(menu_x + 2, menu_y + 7),
        SetForegroundColor(Color::Yellow),
        Print("[2] AI Auto Play Mode (Demo)"),
        cursor::MoveTo(menu_x + 2, menu_y + 8),
        SetForegroundColor(Color::Magenta),
        Print("[3] Imitation Learning (Train from Logs)"),
        cursor::MoveTo(menu_x + 2, menu_y + 9),
        SetForegroundColor(Color::Blue),
        Print("[4] Reinforcement Learning (Self-Play TD)"),
        cursor::MoveTo(menu_x + 2, menu_y + 11),
        SetForegroundColor(Color::Red),
        Print("[Esc] Exit"),
        ResetColor,
        // 現在のモデル状態の表示
        cursor::MoveTo(menu_x, menu_y + 13),
        SetForegroundColor(Color::DarkGrey),
        Print("--- Current Model Weights ---"),
        cursor::MoveTo(menu_x, menu_y + 14),
        Print(format!("MaxHeight: {:.2} | AvgHeight: {:.2} | Bumpy: {:.2} | Holes: {:.2}", model.weights[0], model.weights[1], model.weights[2], model.weights[3])),
        cursor::MoveTo(menu_x, menu_y + 15),
        Print(format!("AbvHoles: {:.2} | Wells: {:.2} | Clr13: {:.2} | Tetrs: {:.2}", model.weights[4], model.weights[5], model.weights[6], model.weights[7])),
        ResetColor
    )?;
    out.flush()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Char('1') => return Ok(1),
                    KeyCode::Char('2') => return Ok(2),
                    KeyCode::Char('3') => return Ok(3),
                    KeyCode::Char('4') => return Ok(4),
                    KeyCode::Esc => return Ok(0),
                    _ => {}
                }
            }
        }
    }
}

// 1. 手動プレイモード
fn run_play_mode(model: &AiModel) -> std::io::Result<()> {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    
    let mut game = Game::new();
    let mut logs: Vec<ExpertStep> = Vec::new();

    // ターン開始時点の状態ログ用
    let mut initial_board = game.board;
    let mut initial_type = game.current_piece.block_type;
    let mut initial_hold = game.hold_piece;
    let mut initial_hold_locked = game.hold_locked;
    let mut initial_next = game.bag.peek_next(5);
    let mut used_hold_this_turn = false;

    let mut last_drop = Instant::now();
    let drop_interval = Duration::from_millis(700);

    ui::draw_game(&game, model, "Human Play (Recording...)", None)?;

    loop {
        let mut piece_locked = false;

        // キーイベントのノンブロッキング処理
        if event::poll(Duration::from_millis(50))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Left => {
                        game.try_move(-1, 0);
                    }
                    KeyCode::Right => {
                        game.try_move(1, 0);
                    }
                    KeyCode::Down => {
                        if game.try_move(0, 1) {
                            last_drop = Instant::now();
                        }
                    }
                    KeyCode::Up | KeyCode::Char('x') => {
                        game.try_rotate(RotationDirection::Clockwise);
                    }
                    KeyCode::Char('z') => {
                        game.try_rotate(RotationDirection::CounterClockwise);
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') => {
                        if game.hold() {
                            used_hold_this_turn = true;
                            // ホールド直後はネクストも変わるため再設定
                            initial_type = game.current_piece.block_type;
                            initial_hold = game.hold_piece;
                            initial_hold_locked = game.hold_locked;
                            initial_next = game.bag.peek_next(5);
                        }
                    }
                    KeyCode::Char(' ') => {
                        // ハードドロップ直前に、配置前の情報を確定させてログへ記録
                        let final_x = game.current_piece.x;
                        let final_rot = game.current_piece.rotation;
                        
                        logs.push(ExpertStep {
                            board: initial_board,
                            current_type: initial_type,
                            hold_piece: initial_hold,
                            hold_locked: initial_hold_locked,
                            next_queue: initial_next.clone(),
                            chosen_x: final_x,
                            chosen_rotation: final_rot,
                            chosen_hold: used_hold_this_turn,
                        });

                        game.hard_drop();
                        piece_locked = true;
                    }
                    KeyCode::Esc => {
                        break;
                    }
                    _ => {}
                }
                ui::draw_game(&game, model, "Human Play (Recording...)", None)?;
            }
        }

        // 自然落下
        if last_drop.elapsed() >= drop_interval {
            if !game.try_move(0, 1) {
                // 下に行けない場合、ハードドロップ同様にログを記録してロック
                let final_x = game.current_piece.x;
                let final_rot = game.current_piece.rotation;

                logs.push(ExpertStep {
                    board: initial_board,
                    current_type: initial_type,
                    hold_piece: initial_hold,
                    hold_locked: initial_hold_locked,
                    next_queue: initial_next.clone(),
                    chosen_x: final_x,
                    chosen_rotation: final_rot,
                    chosen_hold: used_hold_this_turn,
                });

                game.lock_piece();
                piece_locked = true;
            }
            last_drop = Instant::now();
            ui::draw_game(&game, model, "Human Play (Recording...)", None)?;
        }

        if piece_locked {
            if game.game_over {
                // ゲームオーバー表示
                let mut out = stdout();
                queue!(
                    out,
                    cursor::MoveTo(ui::UI_X_OFFSET + 3, ui::UI_Y_OFFSET + 10),
                    SetBackgroundColor(Color::Red),
                    SetForegroundColor(Color::White),
                    Print(" GAME OVER "),
                    ResetColor
                )?;
                out.flush()?;
                std::thread::sleep(Duration::from_millis(1500));
                break;
            }

            // 新しいターンの初期状態を保存
            initial_board = game.board;
            initial_type = game.current_piece.block_type;
            initial_hold = game.hold_piece;
            initial_hold_locked = game.hold_locked;
            initial_next = game.bag.peek_next(5);
            used_hold_this_turn = false;

            ui::draw_game(&game, model, "Human Play (Recording...)", None)?;
        }
    }

    // ログの保存
    if !logs.is_empty() {
        // 既存のログがあれば追記するようにロード
        let mut existing_logs = load_log(DATASET_PATH).unwrap_or_default();
        existing_logs.append(&mut logs);
        save_log(&existing_logs, DATASET_PATH)?;
    }

    Ok(())
}

// 2. AI自動デモプレイモード
fn run_ai_mode(model: &AiModel) -> std::io::Result<()> {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    
    let mut game = Game::new();
    ui::draw_game(&game, model, "AI Auto Play", None)?;

    let step_delay = Duration::from_millis(150);

    loop {
        // キー入力監視 (Escで中断)
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Esc {
                    break;
                }
            }
        }

        // AIの意思決定
        let candidates = enumerate_all_moves(&game, model);
        if candidates.is_empty() {
            game.game_over = true;
        }

        if game.game_over {
            // ゲームオーバー表示
            let mut out = stdout();
            queue!(
                out,
                cursor::MoveTo(ui::UI_X_OFFSET + 3, ui::UI_Y_OFFSET + 10),
                SetBackgroundColor(Color::Red),
                SetForegroundColor(Color::White),
                Print(" GAME OVER "),
                ResetColor
            )?;
            out.flush()?;
            std::thread::sleep(Duration::from_millis(1500));
            break;
        }

        let best_move = &candidates[0];

        // ホールドのアニメーション
        if best_move.use_hold {
            game.hold();
            ui::draw_game(&game, model, "AI Auto Play", None)?;
            std::thread::sleep(step_delay);
        }

        // 回転を合わせるアニメーション
        let target_rot = best_move.final_piece.rotation;
        while game.current_piece.rotation != target_rot {
            game.try_rotate(RotationDirection::Clockwise);
            ui::draw_game(&game, model, "AI Auto Play", None)?;
            std::thread::sleep(Duration::from_millis(50));
        }

        // Xを合わせるアニメーション
        let target_x = best_move.final_piece.x;
        while game.current_piece.x != target_x {
            let dx = if target_x > game.current_piece.x { 1 } else { -1 };
            game.try_move(dx, 0);
            ui::draw_game(&game, model, "AI Auto Play", None)?;
            std::thread::sleep(Duration::from_millis(50));
        }

        // ハードドロップして固定
        game.hard_drop();
        ui::draw_game(&game, model, "AI Auto Play", None)?;
        std::thread::sleep(step_delay);
    }

    Ok(())
}

// 3. 模倣学習実行モード
fn run_imitation_mode(model: &mut AiModel) -> std::io::Result<()> {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    let mut out = stdout();

    let logs = match load_log(DATASET_PATH) {
        Ok(data) => data,
        Err(_) => {
            queue!(
                out,
                cursor::MoveTo(5, 5),
                SetForegroundColor(Color::Red),
                Print("Error: dataset.json not found!"),
                cursor::MoveTo(5, 6),
                SetForegroundColor(Color::White),
                Print("Please play the game first (Mode [1]) to record expert logs."),
                cursor::MoveTo(5, 8),
                Print("Press any key to return to menu..."),
                ResetColor
            )?;
            out.flush()?;
            // キー入力待ち
            loop {
                if event::poll(Duration::from_millis(100))? {
                    let _ = event::read()?;
                    break;
                }
            }
            return Ok(());
        }
    };

    queue!(
        out,
        cursor::MoveTo(5, 2),
        SetForegroundColor(Color::Magenta),
        Print("=== Imitation Learning (Behavioral Cloning) ==="),
        cursor::MoveTo(5, 3),
        SetForegroundColor(Color::White),
        Print(format!("Loaded {} expert transitions.", logs.len())),
        cursor::MoveTo(5, 5),
        Print("Training model with SGD..."),
        ResetColor
    )?;
    out.flush()?;

    let epochs = 50;
    let lr = 0.05;

    for epoch in 1..=epochs {
        let (loss, samples, match_rate) = train_one_epoch(model, &logs, lr);
        
        queue!(
            out,
            cursor::MoveTo(5, 6 + epoch as u16),
            Print(format!("Epoch {:>2}/{}: Loss = {:.4} | Valid Samples = {} | Match Rate = {:>6.2}%", epoch, epochs, loss, samples, match_rate * 100.0))
        )?;
        out.flush()?;
        
        // 学習中の中断キー検知 (Esc)
        if event::poll(Duration::from_millis(10))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Esc {
                    queue!(out, cursor::MoveTo(5, 7 + epoch as u16), SetForegroundColor(Color::Yellow), Print("Training interrupted by user."), ResetColor)?;
                    out.flush()?;
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    save_model(model)?;

    queue!(
        out,
        cursor::MoveTo(5, 10 + epochs as u16),
        SetForegroundColor(Color::Green),
        Print(format!("Successfully saved trained model weights to '{}'.", MODEL_PATH)),
        cursor::MoveTo(5, 11 + epochs as u16),
        SetForegroundColor(Color::White),
        Print("Press any key to return to menu..."),
        ResetColor
    )?;
    out.flush()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            let _ = event::read()?;
            break;
        }
    }

    Ok(())
}

// 4. 強化学習実行モード
fn run_rl_mode(model: &mut AiModel) -> std::io::Result<()> {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    
    let game = Game::new();
    
    // パラメータ
    let alpha = 0.001; // 重みの更新学習率
    let gamma = 0.90;  // 割引率
    let mut epsilon = 0.10; // 探索率
    let min_epsilon = 0.01;
    let epsilon_decay = 0.995;

    let mut ep = 0;
    let mut lines_cleared_history = Vec::new();

    ui::draw_game(&game, model, "Reinforcement Learning (Training...)", Some((ep, 0.0, epsilon)))?;

    loop {
        // キー入力監視 (Escで中断)
        if event::poll(Duration::from_millis(5))? {
            if let Event::Key(key_event) = event::read()? {
                if key_event.code == KeyCode::Esc {
                    break;
                }
            }
        }

        // 1エピソード（1回のゲーム）をバックグラウンドで高速実行
        let (lines, _turns, _reward) = rl::run_rl_episode(model, epsilon, alpha, gamma);
        ep += 1;
        lines_cleared_history.push(lines);

        // 探索率減衰
        epsilon = (epsilon * epsilon_decay).max(min_epsilon);

        // 最新10ゲームの平均消去ライン数
        let window_size = 30.min(lines_cleared_history.len());
        let start_idx = lines_cleared_history.len() - window_size;
        let recent_lines = &lines_cleared_history[start_idx..];
        let avg_lines = (recent_lines.iter().sum::<u32>() as f32) / (window_size as f32);

        // 各エピソード後にUIを再描画（高速で進行するため適度なスロットリングを入れる）
        // 10エピソードごとに描画するか、少しスリープを入れる
        if ep % 5 == 0 {
            // ダミーのゲームを画面描画用に反映
            let draw_game = Game::new(); // 静的な状態でもよい
            ui::draw_game(&draw_game, model, "Reinforcement Learning (Training...)", Some((ep, avg_lines, epsilon)))?;
        }
    }

    save_model(model)?;

    // 終了画面の表示
    let mut out = stdout();
    execute!(out, terminal::Clear(terminal::ClearType::All))?;
    queue!(
        out,
        cursor::MoveTo(5, 5),
        SetForegroundColor(Color::Green),
        Print("Reinforcement Learning Paused and Saved!"),
        cursor::MoveTo(5, 7),
        SetForegroundColor(Color::White),
        Print(format!("Total Trained Episodes: {}", ep)),
        cursor::MoveTo(5, 8),
        Print(format!("Saved model weights to '{}'.", MODEL_PATH)),
        cursor::MoveTo(5, 10),
        Print("Press any key to return to menu..."),
        ResetColor
    )?;
    out.flush()?;

    loop {
        if event::poll(Duration::from_millis(100))? {
            let _ = event::read()?;
            break;
        }
    }

    Ok(())
}
