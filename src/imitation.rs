use crate::tetris::{Game, Board, BlockType, Piece};
use crate::ai::{AiModel, enumerate_all_moves};
use serde::{Serialize, Deserialize};
use std::fs::File;
use std::io::{BufReader, BufWriter};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExpertStep {
    pub board: Board,
    pub current_type: BlockType,
    pub hold_piece: Option<BlockType>,
    pub hold_locked: bool,
    pub next_queue: Vec<BlockType>,
    pub chosen_x: i32,
    pub chosen_rotation: usize,
    pub chosen_hold: bool,
}

pub fn save_log(steps: &[ExpertStep], path: &str) -> std::io::Result<()> {
    let file = File::create(path)?;
    let writer = BufWriter::new(file);
    serde_json::to_writer_pretty(writer, steps)?;
    Ok(())
}

pub fn load_log(path: &str) -> std::io::Result<Vec<ExpertStep>> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    let steps = serde_json::from_reader(reader)?;
    Ok(steps)
}

// 模倣学習（Behavioral Cloning）を1エポック実行する
// 損失関数の勾配を計算し、モデルの重みを更新する
// 戻り値: (平均損失, 有効サンプル数, エキスパート一致率)
pub fn train_one_epoch(
    model: &mut AiModel,
    dataset: &[ExpertStep],
    learning_rate: f32,
) -> (f32, usize, f32) {
    let mut total_loss = 0.0;
    let mut valid_samples = 0;
    let mut matched_predictions = 0; // AIの最高評価手がエキスパートの手と一致した数

    let num_weights = model.weights.len();
    let mut grad_accum = vec![0.0; num_weights];

    for step in dataset {
        // 状態の復元
        let mut game = Game::new();
        game.board = step.board;
        game.current_piece = Piece::new(step.current_type);
        game.hold_piece = step.hold_piece;
        game.hold_locked = step.hold_locked;
        
        // bagのnext_queueを設定（peek_next(1)などでインデックスエラーを防ぐため十分な数を確保）
        let mut bag_queue = step.next_queue.clone();
        if bag_queue.is_empty() {
            // ネクストが空の場合はダミーを詰める
            bag_queue = vec![BlockType::I; 5];
        }
        while bag_queue.len() < 10 {
            bag_queue.push(BlockType::T);
        }
        game.bag.queue = bag_queue;

        // すべての候補手を列挙（AIモデルで評価値算出済み）
        let candidates = enumerate_all_moves(&game, model);
        if candidates.is_empty() {
            continue;
        }

        // エキスパートが選んだ手に対応する候補手を探す
        let target_opt = candidates.iter().position(|c| {
            c.use_hold == step.chosen_hold && 
            c.x == step.chosen_x && 
            c.rotation == step.chosen_rotation
        });

        let target_idx = match target_opt {
            Some(idx) => idx,
            None => {
                // エキスパートの手が、シミュレーターで「有効な配置」として列挙されなかった場合
                // （SRSキックや経路判定のわずかな差異などで発生しうる）
                continue;
            }
        };

        valid_samples += 1;

        // AIの最高評価手がエキスパートと一致しているか
        if target_idx == 0 {
            matched_predictions += 1;
        }

        // ソフトマックスの計算
        // オーバーフローを防ぐため、最大値を差し引く (LSEトリック)
        let max_score = candidates.iter()
            .map(|c| c.eval_score)
            .fold(f32::NEG_INFINITY, f32::max);

        let mut sum_exp = 0.0;
        let mut probs = vec![0.0; candidates.len()];
        for (i, c) in candidates.iter().enumerate() {
            let score_shifted = c.eval_score - max_score;
            let exp_val = score_shifted.exp();
            probs[i] = exp_val;
            sum_exp += exp_val;
        }

        // 確率の正規化
        for i in 0..candidates.len() {
            probs[i] /= sum_exp;
        }

        // 損失: L = -log(P(target))
        let target_prob = probs[target_idx].max(1e-7); // 0ディバイド・log(0)回避
        let loss = -target_prob.ln();
        total_loss += loss;

        // 勾配計算: grad = sum_a P(a)*phi(a) - phi(target)
        for (i, c) in candidates.iter().enumerate() {
            let prob = probs[i];
            for k in 0..num_weights {
                grad_accum[k] += prob * c.features[k];
            }
        }
        let target_features = &candidates[target_idx].features;
        for k in 0..num_weights {
            grad_accum[k] -= target_features[k];
        }
    }

    if valid_samples > 0 {
        // 重みの更新 (勾配降下法)
        // 勾配の平均を取る
        for k in 0..num_weights {
            let grad = grad_accum[k] / (valid_samples as f32);
            model.weights[k] -= learning_rate * grad;
        }
        
        let avg_loss = total_loss / (valid_samples as f32);
        let match_rate = (matched_predictions as f32) / (valid_samples as f32);
        (avg_loss, valid_samples, match_rate)
    } else {
        (0.0, 0, 0.0)
    }
}
