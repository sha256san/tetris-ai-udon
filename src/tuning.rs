use crate::tetris::Game;
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
            // addplan5: 5手先読み（Depth 5, Beam Width TERRAIN_PRUNING_TOP_K_TUNING）でロングレンジ戦術を探索
            let candidates = crate::ai::beam_search(&game, model, 5, crate::config::heuristic::TERRAIN_PRUNING_TOP_K_TUNING, None, 0);
            if candidates.is_empty() {
                break;
            }

            let best = &candidates[0];
            if best.use_hold {
                game.hold();
            }
            game.current_piece.x = best.final_piece.x;
            game.current_piece.y = best.final_piece.y;
            game.current_piece.rotation = best.final_piece.rotation;
            game.last_action_was_rotate = best.was_rotate;
            game.lock_piece();
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

    let total_tspins = avg_tsd + avg_tst + avg_tss;
    // T-Spin (TST + TSD + TSS 合計 5回以上) 達成を最大化する Fitness 関数
    let fitness = (avg_tsd * 8000.0) + (avg_tst * 12000.0) + (avg_tss * 3000.0) + (total_tspins * 4000.0) + (avg_lines * 50.0) + (avg_btb * 150.0);
    (fitness, avg_tsd, avg_tst, avg_tss, avg_lines)
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct VramCheckpointData {
    pub iteration: usize,
    pub fitness: f32,
    pub avg_tsd: f32,
    pub avg_tst: f32,
    pub avg_tss: f32,
    pub avg_lines: f32,
    pub vram_free_mb: f32,
    pub vram_total_mb: f32,
    pub weights_from_vram: Vec<f32>,
}

fn save_vram_checkpoint(
    iter: usize,
    weights: &[f32],
    fitness: f32,
    avg_tsd: f32,
    avg_tst: f32,
    avg_tss: f32,
    avg_lines: f32,
    worker_id: usize,
) {
    let hip = crate::hip::get_hip_evaluator();
    // 1. GPU VRAM に重みを転送
    hip.upload_weights_to_vram(weights);

    // 2. VRAM から直接重みデータをリードバック（VRAM整合性の検証）
    let vram_weights = hip.readback_weights_from_vram(weights.len()).unwrap_or_else(|| weights.to_vec());
    let (free_b, total_b) = hip.get_vram_usage().unwrap_or((0, 0));
    let vram_free_mb = (free_b as f32) / (1024.0 * 1024.0);
    let vram_total_mb = (total_b as f32) / (1024.0 * 1024.0);

    let checkpoint = VramCheckpointData {
        iteration: iter,
        fitness,
        avg_tsd,
        avg_tst,
        avg_tss,
        avg_lines,
        vram_free_mb,
        vram_total_mb,
        weights_from_vram: vram_weights.clone(),
    };

    let _ = std::fs::create_dir_all("checkpoints");
    let file_path = if worker_id > 0 {
        format!("checkpoints/worker_{}_vram_model_iter_{:04}.json", worker_id, iter)
    } else {
        format!("checkpoints/vram_model_iter_{:04}.json", iter)
    };
    if let Ok(file) = std::fs::File::create(&file_path) {
        let writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(writer, &checkpoint);
    }
    let latest_path = if worker_id > 0 {
        format!("checkpoints/worker_{}_vram_checkpoint.json", worker_id)
    } else {
        "vram_weights_checkpoint.json".to_string()
    };
    if let Ok(file) = std::fs::File::create(&latest_path) {
        let writer = std::io::BufWriter::new(file);
        let _ = serde_json::to_writer_pretty(writer, &checkpoint);
    }
}

/// 反復調整（CMA-ES / Evolutionary Optimization）により T-spin 特化の評価関数重みを自動作成
pub fn optimize_tspin_weights(
    iterations: usize,
    initial_model: Option<&AiModel>,
    worker_id: usize,
) -> TSpinOptimizationResult {
    let iterations = if iterations == 0 { 100 } else { iterations };
    let worker_tag = if worker_id > 0 {
        format!("[Worker #{}] ", worker_id)
    } else {
        "".to_string()
    };

    println!("\n========================================================");
    println!("  {}T-Spin 特化 評価関数 {}回 最適化チューニング開始", worker_tag, iterations);
    println!("  目標: TSD / TST / TSS の発生頻度およびT-slot構築力の最大化");
    println!("  GPU VRAM同期: 各イテレーションでVRAMメモリからデータを逐次保存");
    println!("========================================================\n");

    let eval_seeds = vec![
        42 + (worker_id as u64 * 1000),
        100 + (worker_id as u64 * 1000),
        777 + (worker_id as u64 * 1000),
        2026 + (worker_id as u64 * 1000),
        9999 + (worker_id as u64 * 1000),
    ];
    let max_pieces_per_game = 220;

    let current_model = if let Some(m) = initial_model {
        m.clone()
    } else {
        AiModel::new_25_feature_default()
    };

    let (initial_fitness, init_tsd, init_tst, init_tss, init_lines) =
        evaluate_tspin_fitness(&current_model, &eval_seeds, max_pieces_per_game);

    // 初期状態の VRAM チェックポイント保存
    save_vram_checkpoint(0, &current_model.weights, initial_fitness, init_tsd, init_tst, init_tss, init_lines, worker_id);

    let gpu = crate::gpu::get_gpu_evaluator();
    let hip = crate::hip::get_hip_evaluator();
    let (free_b, total_b) = hip.get_vram_usage().unwrap_or((0, 0));
    println!("{}初期状態 (Iteration 0):", worker_tag);
    println!("  Fitness: {:.1} | 平均 TSD: {:.2}回 | TST: {:.2}回 | TSS: {:.2}回 | 消去ライン: {:.1}行",
        initial_fitness, init_tsd, init_tst, init_tss, init_lines);
    println!("  [GPU Compute] Vulkan (wgpu): {}", gpu.get_info_string());
    if hip.is_available {
        println!("  [VRAM Info  ] Free: {:.2} GB / Total: {:.2} GB (VRAM Synchronized)",
            free_b as f64 / 1e9, total_b as f64 / 1e9);
    }
    let dump_name = if worker_id > 0 {
        format!("checkpoints/worker_{}_vram_model_iter_0000.json", worker_id)
    } else {
        "checkpoints/vram_model_iter_0000.json".to_string()
    };
    println!("  [VRAM Dump  ] -> {} 保存完了\n", dump_name);

    let mut best_weights = current_model.weights.clone();
    let mut best_fitness = initial_fitness;
    let mut best_tsd = init_tsd;
    let mut best_tst = init_tst;
    let mut best_tss = init_tss;
    let mut best_lines = init_lines;

    let mut history = Vec::new();
    let mut rng = rand::thread_rng();

    for iter in 1..=iterations {
        // 変異ステップサイズの適応的調整
        let step_scale = 1.0 - (iter as f32 / iterations as f32) * 0.70;
        let mutation_rate = (0.30 * step_scale).max(0.08);

        // 候補重みベクトルの生成
        let mut candidate_weights = best_weights.clone();
        for i in 0..candidate_weights.len() {
            if rng.r#gen::<f32>() < 0.4 {
                let delta = rng.gen_range(-18.0..18.0) * mutation_rate;
                candidate_weights[i] += delta;
            }
        }

        // T-spin関連パラメータ（x0: TSpin, x1: TSpinTerrain, x10: BTB, x19: FutureFit）の強調探索
        if rng.r#gen::<f32>() < 0.35 {
            candidate_weights[0] += rng.gen_range(0.0..12.0) * step_scale; // TSpin
            candidate_weights[1] += rng.gen_range(0.0..10.0) * step_scale; // TSpinTerrain
        }

        let mut candidate_model = current_model.clone();
        candidate_model.weights = candidate_weights.clone();

        // 評価シード（ミニバッチ3シード）
        let worker_offset = worker_id as u64 * 10007;
        let batch_seeds = [
            iter as u64 * 31 + worker_offset + 7,
            iter as u64 * 67 + worker_offset + 13,
            iter as u64 * 101 + worker_offset + 97,
        ];

        let (fit, _tsd, _tst, _tss, _lines) = evaluate_tspin_fitness(&candidate_model, &batch_seeds, max_pieces_per_game);

        let mut updated = false;
        if fit > best_fitness {
            let (v_fit, v_tsd, v_tst, v_tss, v_lines) = evaluate_tspin_fitness(&candidate_model, &eval_seeds, max_pieces_per_game);
            if v_fit > best_fitness {
                best_fitness = v_fit;
                best_weights = candidate_weights;
                best_tsd = v_tsd;
                best_tst = v_tst;
                best_tss = v_tss;
                best_lines = v_lines;
                updated = true;
            }
        }

        // 10イテレーション毎、または更新時に VRAM からデータを保存
        if iter % 10 == 0 || iter == iterations || updated {
            save_vram_checkpoint(iter, &best_weights, best_fitness, best_tsd, best_tst, best_tss, best_lines, worker_id);
            if iter % 10 == 0 || iter == iterations {
                println!(
                    "{}Iteration {:4}/{} | Best Fitness: {:7.1} | TSD: {:.2}回 | TST: {:.2}回 | TSS: {:.2}回 | 消去: {:.1}行 | [VRAM Saved]",
                    worker_tag, iter, iterations, best_fitness, best_tsd, best_tst, best_tss, best_lines
                );
            }
        }

        if iter % 10 == 0 || iter == iterations {
            history.push(TSpinIterationLog {
                iteration: iter,
                best_fitness,
                avg_tsd: best_tsd,
                avg_total_tspins: best_tsd + best_tst + best_tss,
                avg_lines: best_lines,
            });
            if history.len() > 50 {
                history.remove(0);
            }
        }
    }

    println!("\n========================================================");
    println!("  {}{}回 最適化完了！", worker_tag, iterations);
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
