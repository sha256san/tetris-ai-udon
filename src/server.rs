use axum::{
    routing::get,
    Router,
    Json,
    extract::State,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::services::ServeDir;
use crate::tetris::Game;
use crate::ai::{AiModel, enumerate_all_moves};

#[derive(serde::Serialize, Clone)]
pub struct BattleState {
    pub p1: Game,
    pub p1_wins: u32,
    pub p2: Game,
    pub p2_wins: u32,
}

pub async fn start_server() {
    let mut model = AiModel::new_default();
    if let Ok(mut file) = std::fs::File::open(crate::MODEL_PATH) {
        if let Ok(loaded_model) = serde_json::from_reader::<_, AiModel>(&mut file) {
            model = loaded_model;
        }
    }

    let shared_state = Arc::new(Mutex::new(BattleState {
        p1: Game::new(),
        p1_wins: 0,
        p2: Game::new(),
        p2_wins: 0,
    }));

    let state_for_bg = shared_state.clone();
    
    // Background simulation loop
    tokio::spawn(async move {
        // AIが速すぎると見えにくいので、1秒間に約3手 (300ms)
        let mut interval = tokio::time::interval(std::time::Duration::from_millis(300));
        let mut game_over_pause = 0;

        loop {
            interval.tick().await;
            let mut state = state_for_bg.lock().await;

            if game_over_pause > 0 {
                game_over_pause -= 1;
                if game_over_pause == 0 {
                    state.p1 = Game::new();
                    state.p2 = Game::new();
                }
                continue;
            }

            // P1 Move
            state.p1.apply_garbage();
            let candidates1 = enumerate_all_moves(&state.p1, &model, None, 0);
            if candidates1.is_empty() {
                state.p1.game_over = true;
            } else {
                let chosen = &candidates1[0];
                if chosen.use_hold { state.p1.hold(); }
                state.p1.current_piece.x = chosen.final_piece.x;
                state.p1.current_piece.y = chosen.final_piece.y;
                state.p1.current_piece.rotation = chosen.final_piece.rotation;
                
                state.p1.lock_piece();
                let damage = state.p1.last_firepower;
                if damage > 0 {
                    state.p2.pending_garbage += damage;
                }
            }

            // P2 Move
            state.p2.apply_garbage();
            let candidates2 = enumerate_all_moves(&state.p2, &model, None, 0);
            if candidates2.is_empty() {
                state.p2.game_over = true;
            } else {
                let chosen = &candidates2[0];
                if chosen.use_hold { state.p2.hold(); }
                state.p2.current_piece.x = chosen.final_piece.x;
                state.p2.current_piece.y = chosen.final_piece.y;
                state.p2.current_piece.rotation = chosen.final_piece.rotation;
                
                state.p2.lock_piece();
                let damage = state.p2.last_firepower;
                if damage > 0 {
                    state.p1.pending_garbage += damage;
                }
            }

            // Check game over
            if state.p1.game_over || state.p2.game_over {
                if state.p1.game_over && !state.p2.game_over {
                    state.p2_wins += 1;
                } else if state.p2.game_over && !state.p1.game_over {
                    state.p1_wins += 1;
                }
                game_over_pause = 6; // ~1.8 seconds pause before restart
            }
        }
    });

    let app = Router::new()
        .route("/api/state", get(get_state))
        .fallback_service(ServeDir::new("templates"))
        .with_state(shared_state);

    println!("\n===========================================");
    println!("AI Battle Web Server running!");
    println!("Please open: http://localhost:3000/battle/");
    println!("Press CTRL+C to exit.");
    println!("===========================================\n");

    let url = "http://localhost:3000/battle/";
    let _ = std::process::Command::new("python3")
        .arg("-c")
        .arg(format!("import webbrowser; webbrowser.open('{}')", url))
        .status();

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

async fn get_state(State(state): State<Arc<Mutex<BattleState>>>) -> Json<BattleState> {
    let state = state.lock().await.clone();
    Json(state)
}
