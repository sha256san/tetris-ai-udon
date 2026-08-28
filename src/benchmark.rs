use std::time::Instant;
use serde::{Serialize, Deserialize};
use crate::tetris::{Game, BlockType, Piece, BOARD_WIDTH, INTERNAL_HEIGHT};
use crate::ai::{AiModel, beam_search, enumerate_all_moves_base, CandidateMove, GpuBackendSelection};
use rand::SeedableRng;
use rand::rngs::StdRng;
use rand::seq::SliceRandom;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkConfig {
    pub name: String,
    pub depth: usize,
    pub beam_width: usize,
    pub description: String,
    #[serde(default)]
    pub backend: Option<GpuBackendSelection>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameRunResult {
    pub seed: u64,
    pub lines: u32,
    pub score: u32,
    pub pieces: u32,
    pub tetris_clears: u32,
    pub tspins: u32,
    pub duration_sec: f64,
    pub pps: f64,
    pub avg_search_ms: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BenchmarkSummary {
    pub config: BenchmarkConfig,
    pub games_played: usize,
    pub avg_lines: f64,
    pub max_lines: u32,
    pub avg_score: f64,
    pub max_score: u32,
    pub avg_pieces: f64,
    pub avg_tetris_count: f64,
    pub avg_tspin_count: f64,
    pub avg_pps: f64,
    pub avg_search_ms: f64,
    pub overall_score: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MicroBenchmarkResult {
    pub batch_size: usize,
    pub rocm_avg_us: f64,
    pub rocm_meps: f64,
    pub vulkan_avg_us: f64,
    pub vulkan_meps: f64,
    pub cpu_avg_us: f64,
    pub cpu_meps: f64,
    pub speedup_rocm_vs_vulkan: f64,
    pub speedup_rocm_vs_cpu: f64,
}

pub fn create_seeded_game(seed: u64) -> Game {
    let mut rng = StdRng::seed_from_u64(seed);
    let mut queue = Vec::new();
    for _ in 0..1000 {
        let mut bag = BlockType::all().to_vec();
        bag.shuffle(&mut rng);
        queue.extend(bag);
    }
    let first = queue.remove(0);
    Game {
        board: [[None; BOARD_WIDTH]; INTERNAL_HEIGHT],
        current_piece: Piece::new(first),
        bag: crate::tetris::Bag { queue },
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

pub fn run_single_game(
    model: &AiModel,
    config: &BenchmarkConfig,
    seed: u64,
    max_pieces: u32,
) -> GameRunResult {
    let mut game = create_seeded_game(seed);
    let mut piece_count = 0;
    let mut tetris_count = 0;
    let mut tspin_count = 0;
    let mut total_search_duration = std::time::Duration::ZERO;

    let mut configured_model = model.clone();
    if let Some(b) = config.backend {
        configured_model.backend = Some(b);
    }

    let start_time = Instant::now();

    while !game.game_over && piece_count < max_pieces {
        piece_count += 1;

        let search_start = Instant::now();
        let candidates: Vec<CandidateMove> = if config.depth <= 1 {
            enumerate_all_moves_base(&game, &configured_model, None, 0)
        } else {
            beam_search(&game, &configured_model, config.depth, config.beam_width, None, 0)
        };
        total_search_duration += search_start.elapsed();

        if candidates.is_empty() {
            game.game_over = true;
            break;
        }

        let best = &candidates[0];

        if best.use_hold {
            game.hold();
        }
        game.current_piece.x = best.final_piece.x;
        game.current_piece.rotation = best.final_piece.rotation;
        game.current_piece.y = best.final_piece.y;

        let prev_lines = game.lines_cleared;
        game.lock_piece();
        let cleared = game.lines_cleared - prev_lines;

        if cleared == 4 {
            tetris_count += 1;
        }
        if game.last_t_spin.is_some() {
            tspin_count += 1;
        }
    }

    let elapsed = start_time.elapsed().as_secs_f64();
    let pps = if elapsed > 0.0 { piece_count as f64 / elapsed } else { 0.0 };
    let avg_search_ms = if piece_count > 0 {
        (total_search_duration.as_secs_f64() * 1000.0) / piece_count as f64
    } else {
        0.0
    };

    GameRunResult {
        seed,
        lines: game.lines_cleared,
        score: game.score,
        pieces: piece_count,
        tetris_clears: tetris_count,
        tspins: tspin_count,
        duration_sec: elapsed,
        pps,
        avg_search_ms,
    }
}

/// ROCm vs Vulkan vs CPU のマイクロベンチマーク（純粋なディスパッチ遅延・スループット比較）
pub fn run_micro_benchmark(num_features: usize, is_nonlinear: bool) -> Vec<MicroBenchmarkResult> {
    let batch_sizes = vec![10, 50, 100, 250, 500, 1000, 2500, 5000];
    let weights: Vec<f32> = (0..num_features).map(|i| (i as f32 + 1.0) * 0.1).collect();

    let mut results = Vec::new();

    let hip_eval = crate::hip::get_hip_evaluator();
    let gpu_eval = crate::gpu::get_gpu_evaluator();

    println!("\n========================================================");
    println!("  ROCm (HIP) vs Vulkan (wgpu) マイクロベンチマーク");
    println!("  特徴量数: {} 次元 | 非線形多項式評価: {}", num_features, is_nonlinear);
    println!("  ROCm: {}", hip_eval.device_name);
    println!("  Vulkan: {}", gpu_eval.get_info_string());
    println!("========================================================\n");

    for &batch_size in &batch_sizes {
        let feature_batch: Vec<Vec<f32>> = (0..batch_size)
            .map(|_| (0..num_features).map(|j| (j as f32 * 0.05).sin()).collect())
            .collect();

        let iterations = if batch_size >= 1000 { 500 } else { 2000 };

        // 1. ROCm HIP
        for _ in 0..50 {
            let _ = hip_eval.evaluate_batch(&weights, &feature_batch, is_nonlinear);
        }
        let start_rocm = Instant::now();
        for _ in 0..iterations {
            let _ = hip_eval.evaluate_batch(&weights, &feature_batch, is_nonlinear);
        }
        let rocm_total_us = start_rocm.elapsed().as_micros() as f64;
        let rocm_avg_us = rocm_total_us / iterations as f64;
        let rocm_meps = (batch_size as f64 * iterations as f64) / (rocm_total_us * 1e-6) / 1e6;

        // 2. Vulkan wgpu
        for _ in 0..50 {
            let _ = gpu_eval.evaluate_batch(&weights, &feature_batch, is_nonlinear);
        }
        let start_vulkan = Instant::now();
        for _ in 0..iterations {
            let _ = gpu_eval.evaluate_batch(&weights, &feature_batch, is_nonlinear);
        }
        let vulkan_total_us = start_vulkan.elapsed().as_micros() as f64;
        let vulkan_avg_us = vulkan_total_us / iterations as f64;
        let vulkan_meps = (batch_size as f64 * iterations as f64) / (vulkan_total_us * 1e-6) / 1e6;

        // 3. CPU (Single-thread / Rayon)
        let cpu_iterations = iterations.min(500);
        let start_cpu = Instant::now();
        for _ in 0..cpu_iterations {
            let _: Vec<f32> = feature_batch.iter().map(|feats| {
                let mut s = 0.0f32;
                for i in 0..num_features { s += weights[i] * feats[i]; }
                s
            }).collect();
        }
        let cpu_total_us = start_cpu.elapsed().as_micros() as f64;
        let cpu_avg_us = cpu_total_us / cpu_iterations as f64;
        let cpu_meps = (batch_size as f64 * cpu_iterations as f64) / (cpu_total_us * 1e-6) / 1e6;

        let speedup_vulkan = vulkan_avg_us / rocm_avg_us;
        let speedup_cpu = cpu_avg_us / rocm_avg_us;

        println!(
            "Batch {:>4} | ROCm: {:>7.2} μs ({:>6.2} M/s) | Vulkan: {:>7.2} μs ({:>6.2} M/s) | CPU: {:>7.2} μs | ROCm優位性: {:>5.2}x vs Vulkan, {:>5.2}x vs CPU",
            batch_size, rocm_avg_us, rocm_meps, vulkan_avg_us, vulkan_meps, cpu_avg_us, speedup_vulkan, speedup_cpu
        );

        results.push(MicroBenchmarkResult {
            batch_size,
            rocm_avg_us,
            rocm_meps,
            vulkan_avg_us,
            vulkan_meps,
            cpu_avg_us,
            cpu_meps,
            speedup_rocm_vs_vulkan: speedup_vulkan,
            speedup_rocm_vs_cpu: speedup_cpu,
        });
    }

    results
}

pub fn run_full_benchmark(
    model: &AiModel,
    configs: &[BenchmarkConfig],
    seeds: &[u64],
    max_pieces_per_game: u32,
) -> Vec<BenchmarkSummary> {
    let mut summaries = Vec::new();

    println!("\n========================================================");
    println!("  TETRIS AI 探索アルゴリズム 総合ベンチマーク実行中");
    println!("  ROCm HIP Compute: {}", crate::hip::get_hip_evaluator().device_name);
    println!("  Vulkan Compute: {}", crate::gpu::get_gpu_evaluator().get_info_string());
    println!("  テストシード数: {} / 1ゲーム最大ミノ数: {}", seeds.len(), max_pieces_per_game);
    println!("========================================================\n");

    for config in configs {
        print!("▶ 評価中: {:<40} ... ", config.name);
        std::io::Write::flush(&mut std::io::stdout()).unwrap();

        let mut results = Vec::new();
        for &seed in seeds {
            let res = run_single_game(model, config, seed, max_pieces_per_game);
            results.push(res);
        }

        let n = results.len() as f64;
        let avg_lines = results.iter().map(|r| r.lines as f64).sum::<f64>() / n;
        let max_lines = results.iter().map(|r| r.lines).max().unwrap_or(0);
        let avg_score = results.iter().map(|r| r.score as f64).sum::<f64>() / n;
        let max_score = results.iter().map(|r| r.score).max().unwrap_or(0);
        let avg_pieces = results.iter().map(|r| r.pieces as f64).sum::<f64>() / n;
        let avg_tetris = results.iter().map(|r| r.tetris_clears as f64).sum::<f64>() / n;
        let avg_tspin = results.iter().map(|r| r.tspins as f64).sum::<f64>() / n;
        let avg_pps = results.iter().map(|r| r.pps).sum::<f64>() / n;
        let avg_search_ms = results.iter().map(|r| r.avg_search_ms).sum::<f64>() / n;

        let strength_score = avg_lines;
        let efficiency_score = if avg_search_ms > 0.0 { (avg_lines / avg_search_ms).min(500.0) } else { 0.0 };
        let overall_score = strength_score * 0.5 + avg_pps * 2.0 + efficiency_score * 0.3;

        println!("完了! (Avg Lines: {:.1}, Avg PPS: {:.1}, Search: {:.2}ms)", avg_lines, avg_pps, avg_search_ms);

        summaries.push(BenchmarkSummary {
            config: config.clone(),
            games_played: results.len(),
            avg_lines,
            max_lines,
            avg_score,
            max_score,
            avg_pieces,
            avg_tetris_count: avg_tetris,
            avg_tspin_count: avg_tspin,
            avg_pps,
            avg_search_ms,
            overall_score,
        });
    }

    summaries.sort_by(|a, b| b.overall_score.partial_cmp(&a.overall_score).unwrap_or(std::cmp::Ordering::Equal));
    summaries
}
