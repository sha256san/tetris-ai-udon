use crate::tetris::{Game, BlockType};
use crate::ai::AiModel;
use rand::Rng;
use rayon::prelude::*;

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TSpinOptimizationResult {
    pub best_weights: Vec<f32>,
    pub initial_fitness: f32,
    pub best_fitness: f32,
    pub avg_tsd_per_game: f32,
    pub avg_tst_per_game: f32,
    pub avg_tss_per_game: f32,
    pub avg_lines_per_game: f32,
    pub total_iterations: usize,
    pub history: Vec<TSpinIterationLog>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct TSpinIterationLog {
    pub iteration: usize,
    pub best_fitness: f32,
    pub avg_tsd: f32,
    pub avg_total_tspins: f32,
    pub avg_lines: f32,
}

/// 1つのモデル重みセットに対してシード列でゲームを実行し、T-Spin特化のFitnessスコアを計測
pub fn evaluate_tspin_fitness(model: &AiModel, seeds: &[u64], max_pieces: usize) -> (f32, f32, f32, f32, f32) {
    let results: Vec<(u32, u32, u32, u32, u32, u32)> = seeds.par_iter().map(|&seed| {
        let mut game = Game::new_with_seed(seed);
        let mut tsd = 0;
        let mut tst = 0;
        let mut tss = 0;
        let mut btb_count = 0;
        let mut tsd_setups = 0;
        let mut pieces = 0;

        while !game.game_over && pieces < max_pieces {
            let candidates = crate::ai::enumerate_all_moves_base(&game, model, None, 0);
            if candidates.is_empty() {
                break;
            }

            let best = &candidates[0];
            if best.use_hold {
                game.hold();
            }
            game.current_piece.x = best.final_piece.x;
            game.current_piece.rotation = best.final_piece.rotation;
            if best.final_piece.block_type == BlockType::T {
                game.last_action_was_rotate = true;
            }
            game.hard_drop();
            pieces += 1;

            if let Some(ref name) = game.last_t_spin {
                if name.contains("Double") {
                    tsd += 1;
                } else if name.contains("Triple") {
                    tst += 1;
                } else if name.contains("Single") {
                    tss += 1;
                }
            }
            if game.btb {
                btb_count += 1;
            }
            let t_slots = crate::tetris::count_t_slots(&game.board);
            if t_slots > 0 {
                tsd_setups += t_slots as u32;
            }
        }

        (tsd, tst, tss, game.lines_cleared, btb_count, tsd_setups)
    }).collect();

    let n = results.len() as f32;
    let avg_tsd = results.iter().map(|r| r.0 as f32).sum::<f32>() / n;
    let avg_tst = results.iter().map(|r| r.1 as f32).sum::<f32>() / n;
    let avg_tss = results.iter().map(|r| r.2 as f32).sum::<f32>() / n;
    let avg_lines = results.iter().map(|r| r.3 as f32).sum::<f32>() / n;
    let avg_btb = results.iter().map(|r| r.4 as f32).sum::<f32>() / n;
    let avg_setups = results.iter().map(|r| r.5 as f32).sum::<f32>() / n;

    // T-Spin & T-Slot 構築重視の Fitness 関数 (addplan.md 準拠)
    let fitness = avg_tsd * 2500.0 + avg_tst * 3500.0 + avg_tss * 1000.0 + avg_setups * 150.0 + avg_btb * 120.0 + avg_lines * 25.0;
    (fitness, avg_tsd, avg_tst, avg_tss, avg_lines)
}

/// 1000回の反復調整（CMA-ES / Evolutionary Optimization）により T-spin 特化の評価関数重みを自動作成
pub fn optimize_tspin_weights(iterations: usize) -> TSpinOptimizationResult {
    println!("\n========================================================");
    println!("  T-Spin 特化 評価関数 1000回 最適化チューニング開始");
    println!("  目標: TSD / TST / TSS の発生頻度およびT-slot構築力の最大化");
    println!("========================================================\n");

    let eval_seeds = vec![42, 100, 777, 2026, 9999];
    let max_pieces_per_game = 150;

    let mut current_model = AiModel::new_20_feature_default();
    // T-Spin重視の初期バイアスを設定
    current_model.weights[0] = 80.0;   // TSpin (Single/Double/Triple)
    current_model.weights[1] = 60.0;   // TSpinTerrain (T-slots, overhang)
    current_model.weights[2] = -30.0;  // Holes
    current_model.weights[3] = -15.0;  // HoleSpread
    current_model.weights[4] = 20.0;   // PlacementQuality
    current_model.weights[5] = 90.0;   // Tetris
    current_model.weights[6] = -25.0;  // PureSingle (単発消去ペナルティ)
    current_model.weights[7] = -15.0;  // PureDouble
    current_model.weights[8] = -10.0;  // PureTriple
    current_model.weights[9] = 25.0;   // REN
    current_model.weights[10] = 40.0;  // BTB
    current_model.weights[11] = 15.0;  // MaxCombo
    current_model.weights[12] = 10.0;  // MeanCombo
    current_model.weights[13] = 100.0; // PC
    current_model.weights[14] = -15.0; // Height
    current_model.weights[15] = -20.0; // MaxHeight
    current_model.weights[16] = -12.0; // Bumpiness
    current_model.weights[17] = 25.0;  // WellQuality
    current_model.weights[18] = -20.0; // Overhang
    current_model.weights[19] = 35.0;  // FutureFit

    let (initial_fitness, init_tsd, init_tst, init_tss, init_lines) =
        evaluate_tspin_fitness(&current_model, &eval_seeds, max_pieces_per_game);

    println!("初期状態 (Iteration 0):");
    println!("  Fitness: {:.1} | 平均 TSD: {:.2}回 | TST: {:.2}回 | TSS: {:.2}回 | 消去ライン: {:.1}行\n",
        initial_fitness, init_tsd, init_tst, init_tss, init_lines);

    let mut best_weights = current_model.weights.clone();
    let mut best_fitness = initial_fitness;
    let mut best_tsd = init_tsd;
    let mut best_tst = init_tst;
    let mut best_tss = init_tss;
    let mut best_lines = init_lines;

    let mut history = Vec::new();
    let mut rng = rand::thread_rng();

    for iter in 1..=iterations {
        // 変異ステップサイズの適応的調整 (前半は大きく探索、後半は微調整)
        let step_scale = 1.0 - (iter as f32 / iterations as f32) * 0.75;
        let mutation_rate = (0.25 * step_scale).max(0.05);

        // 候補重みベクトルの生成 (上位から変異)
        let mut candidate_weights = best_weights.clone();
        for i in 0..candidate_weights.len() {
            if rng.r#gen::<f32>() < 0.4 {
                let delta = rng.gen_range(-15.0..15.0) * mutation_rate;
                candidate_weights[i] += delta;
            }
        }

        // T-spin関連パラメータ（x0, x1, x10, x19）を積極的に強調探索
        if rng.r#gen::<f32>() < 0.3 {
            candidate_weights[0] += rng.gen_range(0.0..10.0) * step_scale; // TSpin
            candidate_weights[1] += rng.gen_range(0.0..8.0) * step_scale;  // TSpinTerrain
        }

        let mut candidate_model = current_model.clone();
        candidate_model.weights = candidate_weights.clone();

        // 評価シード（ミニバッチ3シード）
        let batch_seeds = [
            iter as u64 * 31 + 7,
            iter as u64 * 67 + 13,
            iter as u64 * 101 + 97,
        ];

        let (fit, _tsd, _tst, _tss, _lines) = evaluate_tspin_fitness(&candidate_model, &batch_seeds, max_pieces_per_game);

        if fit > best_fitness {
            // 本格シード列で再検証
            let (v_fit, v_tsd, v_tst, v_tss, v_lines) = evaluate_tspin_fitness(&candidate_model, &eval_seeds, max_pieces_per_game);
            if v_fit > best_fitness {
                best_fitness = v_fit;
                best_weights = candidate_weights;
                best_tsd = v_tsd;
                best_tst = v_tst;
                best_tss = v_tss;
                best_lines = v_lines;
            }
        }

        if iter % 100 == 0 || iter == iterations {
            println!(
                "Iteration {:4}/{} | Best Fitness: {:7.1} | TSD: {:.2}回 | TST: {:.2}回 | TSS: {:.2}回 | 消去: {:.1}行",
                iter, iterations, best_fitness, best_tsd, best_tst, best_tss, best_lines
            );
            history.push(TSpinIterationLog {
                iteration: iter,
                best_fitness,
                avg_tsd: best_tsd,
                avg_total_tspins: best_tsd + best_tst + best_tss,
                avg_lines: best_lines,
            });
        }
    }

    println!("\n========================================================");
    println!("  1000回 最適化完了！");
    println!("  最適化前 Fitness: {:.1} -> 最適化後 Fitness: {:.1} (+{:.1}%)",
        initial_fitness, best_fitness, ((best_fitness - initial_fitness) / initial_fitness.max(1.0)) * 100.0);
    println!("  最適化後 平均TSD: {:.2}回 / ゲーム | 平均消去: {:.1}行", best_tsd, best_lines);
    println!("========================================================\n");

    TSpinOptimizationResult {
        best_weights,
        initial_fitness,
        best_fitness,
        avg_tsd_per_game: best_tsd,
        avg_tst_per_game: best_tst,
        avg_tss_per_game: best_tss,
        avg_lines_per_game: best_lines,
        total_iterations: iterations,
        history,
    }
}
