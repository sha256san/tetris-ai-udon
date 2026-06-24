mod tetris;
mod ai;
mod imitation;
mod rl;
mod ui;
mod config;
mod opening;

use std::fs::{self, File};
use std::io::{stdout, Write};
use std::path::PathBuf;
use std::time::{Duration, Instant};
use crossterm::{
    execute, queue, cursor, terminal,
    event::{self, Event, KeyCode},
    style::{Color, Print, ResetColor, SetForegroundColor, SetBackgroundColor},
};
use tetris::{Game, RotationDirection};
use ai::{AiModel, enumerate_all_moves};
use imitation::{ExpertStep, save_log, load_log, train_one_epoch};
use opening::OpeningTemplate;

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
    let mut active_opening: Option<OpeningTemplate> = None;

    loop {
        let selection = show_menu(&model, active_opening.as_ref())?;
        match selection {
            1 => run_play_mode(&mut model)?,
            2 => run_ai_mode(&model, active_opening.as_ref())?,
            3 => run_imitation_mode(&mut model)?,
            4 => run_rl_mode(&mut model)?,
            5 => run_load_template_mode(&mut model, &mut active_opening)?,
            6 => run_opening_editor()?,
            _ => break, // Exit
        }
    }

    // ターミナルの復元
    ui::restore_terminal()?;
    Ok(())
}

// メインメニューの表示と選択
fn show_menu(model: &AiModel, active_opening: Option<&OpeningTemplate>) -> std::io::Result<u8> {
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
        Print("             TETRIS AI SYSTEM            "),
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
        cursor::MoveTo(menu_x + 2, menu_y + 10),
        SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }),
        Print("[5] Load / Set Template or Opening"),
        cursor::MoveTo(menu_x + 2, menu_y + 11),
        SetForegroundColor(Color::Cyan),
        Print("[6] Opening Editor (Open Browser)"),
        cursor::MoveTo(menu_x + 2, menu_y + 13),
        SetForegroundColor(Color::Red),
        Print("[Esc] Exit"),
        ResetColor,
        // オープニング状態の表示
        cursor::MoveTo(menu_x, menu_y + 14),
        SetForegroundColor(Color::Rgb { r: 100, g: 200, b: 255 }),
        Print(format!("Opening: {}",
            active_opening.map_or("None (normal mode)".to_string(), |o| {
                format!("{} (active until {} lines)", o.name, o.active_until_lines)
            })
        )),
        // 現在のモデル状態の表示
        cursor::MoveTo(menu_x, menu_y + 15),
        SetForegroundColor(Color::DarkGrey),
        Print("--- Current Model Weights ---"),
        cursor::MoveTo(menu_x, menu_y + 16),
        Print(format!("MaxHeight: {:.2} | AvgHeight: {:.2} | Bumpy: {:.2} | Holes: {:.2}", model.weights[0], model.weights[1], model.weights[2], model.weights[3])),
        cursor::MoveTo(menu_x, menu_y + 17),
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
                    KeyCode::Char('5') => return Ok(5),
                    KeyCode::Char('6') => return Ok(6),
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

    ui::draw_game(&game, model, "Human Play (Recording...)", None, None, 0)?;

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
                ui::draw_game(&game, model, "Human Play (Recording...)", None, None, 0)?;
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
            ui::draw_game(&game, model, "Human Play (Recording...)", None, None, 0)?;
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

            ui::draw_game(&game, model, "Human Play (Recording...)", None, None, 0)?;
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
fn run_ai_mode(model: &AiModel, opening: Option<&OpeningTemplate>) -> std::io::Result<()> {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    
    let mut game = Game::new();
    let mut opening_turn: usize = 0;  // オープニングシーケンスの現在の手番
    ui::draw_game(&game, model, "AI Auto Play", None, opening, opening_turn)?;

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

        // AIの意思決定（オープニングターンを渡す）
        let candidates = enumerate_all_moves(&game, model, opening, opening_turn);
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
            ui::draw_game(&game, model, "AI Auto Play", None, opening, opening_turn)?;
            std::thread::sleep(step_delay);
        }

        // 回転を合わせるアニメーション
        let target_rot = best_move.final_piece.rotation;
        while game.current_piece.rotation != target_rot {
            game.try_rotate(RotationDirection::Clockwise);
            ui::draw_game(&game, model, "AI Auto Play", None, opening, opening_turn)?;
            std::thread::sleep(Duration::from_millis(50));
        }

        // Xを合わせるアニメーション
        let target_x = best_move.final_piece.x;
        while game.current_piece.x != target_x {
            let dx = if target_x > game.current_piece.x { 1 } else { -1 };
            game.try_move(dx, 0);
            ui::draw_game(&game, model, "AI Auto Play", None, opening, opening_turn)?;
            std::thread::sleep(Duration::from_millis(50));
        }

        // ハードドロップして固定
        game.hard_drop();
        ui::draw_game(&game, model, "AI Auto Play", None, opening, opening_turn)?;
        std::thread::sleep(step_delay);

        // オープニングシーケンスが有効な間はターンを進める
        if let Some(op) = opening {
            let max_turns = if let Some(branch) = op.get_active_branch(&game) {
                branch.parsed_placements.len()
            } else {
                op.parsed_placements.len()
            };
            if game.lines_cleared < op.active_until_lines && opening_turn < max_turns {
                opening_turn += 1;
            }
        }
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

    ui::draw_game(&game, model, "Reinforcement Learning (Training...)", Some((ep, 0.0, epsilon)), None, 0)?;

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
            ui::draw_game(&draw_game, model, "Reinforcement Learning (Training...)", Some((ep, avg_lines, epsilon)), None, 0)?;
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

// 5. テンプレート読み込みモード（重みテンプレ & オープニングテンプレ）
fn run_load_template_mode(model: &mut AiModel, active_opening: &mut Option<OpeningTemplate>) -> std::io::Result<()> {
    execute!(stdout(), terminal::Clear(terminal::ClearType::All))?;
    let mut out = stdout();

    // templates/*.json — 重みテンプレート
    let mut weight_templates: Vec<PathBuf> = fs::read_dir("templates")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.is_file() && p.extension().map_or(false, |ext| ext == "json"))
                .collect()
        })
        .unwrap_or_default();
    weight_templates.sort();

    // templates/openings/*.json — オープニングテンプレート
    let mut opening_templates: Vec<PathBuf> = fs::read_dir("templates/openings")
        .map(|entries| {
            entries
                .filter_map(|e| e.ok())
                .map(|e| e.path())
                .filter(|p| p.extension().map_or(false, |ext| ext == "json"))
                .collect()
        })
        .unwrap_or_default();
    opening_templates.sort();

    // --- 表示 ---
    queue!(
        out,
        cursor::MoveTo(5, 1),
        SetForegroundColor(Color::Rgb { r: 255, g: 165, b: 0 }),
        Print("=== Load Template or Opening ==="),
        cursor::MoveTo(5, 3),
        SetForegroundColor(Color::Cyan),
        Print("[Weight Templates] (overwrites model.json)"),
        ResetColor
    )?;

    let mut row: u16 = 4;
    for (i, path) in weight_templates.iter().enumerate() {
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        queue!(
            out,
            cursor::MoveTo(7, row),
            SetForegroundColor(Color::Green),
            Print(format!("[{}] {}", i + 1, name)),
            ResetColor
        )?;
        row += 1;
    }

    row += 1;
    queue!(
        out,
        cursor::MoveTo(5, row),
        SetForegroundColor(Color::Rgb { r: 150, g: 100, b: 255 }),
        Print("[Opening Templates] (sets opening strategy for AI mode)"),
        ResetColor
    )?;
    row += 1;

    let opening_start_idx = weight_templates.len() + 1; // 1-indexed
    for (i, path) in opening_templates.iter().enumerate() {
        let name = path.file_stem().unwrap_or_default().to_string_lossy();
        let key_num = opening_start_idx + i;
        queue!(
            out,
            cursor::MoveTo(7, row),
            SetForegroundColor(Color::Rgb { r: 180, g: 140, b: 255 }),
            Print(format!("[{}] {}", key_num, name)),
            ResetColor
        )?;
        row += 1;
    }

    // オープニングクリア
    let clear_key = opening_start_idx + opening_templates.len();
    queue!(
        out,
        cursor::MoveTo(7, row),
        SetForegroundColor(Color::Yellow),
        Print(format!("[{}] Clear opening (return to normal mode)", clear_key)),
        ResetColor
    )?;
    row += 2;

    queue!(
        out,
        cursor::MoveTo(5, row),
        SetForegroundColor(Color::Red),
        Print("[Esc] Cancel"),
        ResetColor
    )?;
    out.flush()?;

    // --- キー入力処理 ---
    loop {
        if event::poll(Duration::from_millis(100))? {
            if let Event::Key(key_event) = event::read()? {
                match key_event.code {
                    KeyCode::Esc => return Ok(()),

                    KeyCode::Char(c) if c.is_ascii_digit() => {
                        let num: usize = (c as u8 - b'0') as usize;
                        if num == 0 { continue; }

                        // 重みテンプレートの選択
                        if num >= 1 && num <= weight_templates.len() {
                            let path = &weight_templates[num - 1];
                            match File::open(path) {
                                Ok(file) => {
                                    let reader = std::io::BufReader::new(file);
                                    match serde_json::from_reader::<_, AiModel>(reader) {
                                        Ok(loaded) if loaded.weights.len() == 8 => {
                                            *model = loaded;
                                            save_model(model)?;
                                            let name = path.file_stem().unwrap_or_default().to_string_lossy();
                                            show_confirm_msg(
                                                &mut out,
                                                &format!("Loaded weight template: '{}'", name),
                                                Color::Green,
                                            )?;
                                        }
                                        _ => show_confirm_msg(&mut out, "Error: invalid weight template (need 8 weights).", Color::Red)?,
                                    }
                                }
                                Err(e) => show_confirm_msg(&mut out, &format!("Error: {}", e), Color::Red)?,
                            }
                            wait_any_key()?;
                            return Ok(());
                        }

                        // オープニングテンプレートの選択
                        let op_idx = num - opening_start_idx;
                        if op_idx < opening_templates.len() {
                            let path = &opening_templates[op_idx];
                            match opening::load_opening(path.to_str().unwrap_or("")) {
                                Ok(tmpl) => {
                                    let name = tmpl.name.clone();
                                    let until = tmpl.active_until_lines;
                                    *active_opening = Some(tmpl);
                                    show_confirm_msg(
                                        &mut out,
                                        &format!("Opening set: '{}' (active until {} lines)", name, until),
                                        Color::Rgb { r: 180, g: 140, b: 255 },
                                    )?;
                                }
                                Err(e) => show_confirm_msg(&mut out, &format!("Error loading opening: {}", e), Color::Red)?,
                            }
                            wait_any_key()?;
                            return Ok(());
                        }

                        // オープニングクリア
                        if num == clear_key {
                            *active_opening = None;
                            show_confirm_msg(&mut out, "Opening cleared. AI will use normal evaluation.", Color::Yellow)?;
                            wait_any_key()?;
                            return Ok(());
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}

fn show_confirm_msg(out: &mut std::io::Stdout, msg: &str, color: Color) -> std::io::Result<()> {
    execute!(out, terminal::Clear(terminal::ClearType::All))?;
    queue!(
        out,
        cursor::MoveTo(5, 5),
        SetForegroundColor(color),
        Print(msg),
        cursor::MoveTo(5, 7),
        SetForegroundColor(Color::White),
        Print("Press any key to return..."),
        ResetColor
    )?;
    out.flush()?;
    Ok(())
}

fn wait_any_key() -> std::io::Result<()> {
    loop {
        if event::poll(Duration::from_millis(100))? {
            let _ = event::read()?;
            break;
        }
    }
    Ok(())
}

fn run_opening_editor() -> std::io::Result<()> {
    // ターミナルの状態を一度復元
    ui::restore_terminal()?;

    println!("=========================================");
    println!("       TETRIS OPENING BOARD EDITOR       ");
    println!("=========================================");
    println!("Opening editor in your web browser...");

    let path = std::env::current_dir()?.join("templates/openings/editor.html");
    let file_url = format!("file://{}", path.display());
    let mut opened = false;

    // 1. Python の webbrowser モジュールを試す
    let python_status = std::process::Command::new("python3")
        .arg("-c")
        .arg(format!("import webbrowser; webbrowser.open('{}')", file_url))
        .status();

    if let Ok(s) = python_status {
        if s.success() {
            opened = true;
            println!("Browser opened successfully via Python!");
        }
    }

    // 2. Python が失敗した場合は xdg-open を試す
    if !opened {
        let xdg_status = std::process::Command::new("xdg-open")
            .arg(&path)
            .status();

        if let Ok(s) = xdg_status {
            if s.success() {
                opened = true;
                println!("Browser opened successfully via xdg-open!");
            }
        }
    }

    if !opened {
        println!("\n[Notice] Failed to open browser automatically.");
        println!("Please manually open this path in your browser:");
        println!("{}", path.display());
    }

    println!("\nPress [Enter] to return to Tetris AI menu...");
    let mut input = String::new();
    std::io::stdin().read_line(&mut input)?;

    // ターミナルを再度ゲーム用に戻す
    ui::init_terminal()?;
    Ok(())
}
