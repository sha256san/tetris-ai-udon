use crate::tetris::{Board, BlockType, BOARD_WIDTH, INTERNAL_HEIGHT, get_piece_offsets};
use serde::{Serialize, Deserialize};
use std::collections::{HashSet, VecDeque};

// --------------------------------------------------------------------------
// ミノの配置情報（board_map から自動解析して作成）
// --------------------------------------------------------------------------

#[derive(Debug, Clone)]
pub struct ParsedPlacement {
    pub piece: BlockType,
    /// ミノの最左列（0-indexed）
    pub col: i32,
    /// 回転状態（0〜3）
    pub rotation: usize,
}

// --------------------------------------------------------------------------
// オープニングテンプレート本体
// --------------------------------------------------------------------------

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningBranch {
    /// 条件式リスト (例: ["L < J", "I < Z"])。すべて満たす必要がある(AND)
    #[serde(default)]
    pub conditions: Vec<String>,
    /// 盤面テキスト。各行10文字、ミノ文字か '0'
    #[serde(default)]
    pub board_map: Vec<String>,
    /// 複数巡目の盤面マップリスト。
    #[serde(default)]
    pub board_maps: Vec<Vec<String>>,

    // ---- 解析済みデータ（ロード後に自動設定） ----------------------
    #[serde(skip)]
    pub parsed_placements: Vec<ParsedPlacement>,
    /// board_map から計算した目標列高さ
    #[serde(skip)]
    pub target_heights: Vec<i32>,

    /// 各巡目ごとの配置リスト
    #[serde(skip)]
    pub parsed_placements_list: Vec<Vec<ParsedPlacement>>,
    /// 各巡目ごとの目標列高さ
    #[serde(skip)]
    pub target_heights_list: Vec<Vec<i32>>,
}

/// オープニングテンプレート。templates/openings/*.json から読み込む。
///
/// ## board_map フォーマット
/// 各行は 10 文字（s/z/i/l/j/t/o または 0）。大文字小文字不問。
/// 配列の先頭が上段、末尾が底に近い行。
///
/// 例（63積み）:
/// ```json
/// "board_map": [
///   "s000ti0000",
///   "ss0tti0oo0",
///   "jszzti0ool",
///   "jjjzzi0lll"
/// ]
/// ```
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpeningTemplate {
    /// テンプレート名
    pub name: String,
    /// 説明文
    pub description: String,

    // ---- board_map 形式（推奨・分岐なし用） ------------------------
    /// 盤面テキスト。各行 10 文字、ミノ文字か '0'。
    #[serde(default)]
    pub board_map: Vec<String>,
    /// 複数巡目の盤面マップリスト（推奨・分岐なし用）
    #[serde(default)]
    pub board_maps: Vec<Vec<String>>,

    // ---- 条件分岐付きブランチ -------------------------------------
    /// 条件ごとのマップと配置情報
    #[serde(default)]
    pub branches: Vec<OpeningBranch>,

    // ---- フェーズ管理 -----------------------------------------------
    /// 何ライン消去したらオープニング終了
    pub active_until_lines: u32,

    // ---- 評価値 -----------------------------------------------------
    /// ミノ配置一致 1 手あたりのボーナス
    #[serde(default = "default_sequence_bonus")]
    pub sequence_match_bonus: f32,
    /// 目標列高さへの近さボーナス（1 列あたり）
    #[serde(default = "default_height_bonus")]
    pub opening_bonus_per_column: f32,
    /// オープニング中に使用する AI 重み（8 要素）
    pub opening_weights: Vec<f32>,

    // ---- 解析済みデータ（ロード後に自動設定） ----------------------
    #[serde(skip)]
    pub parsed_placements: Vec<ParsedPlacement>,
    /// board_map から計算した目標列高さ
    #[serde(skip)]
    pub target_heights: Vec<i32>,

    /// 複数巡目の解析済み配置リスト
    #[serde(skip)]
    pub parsed_placements_list: Vec<Vec<ParsedPlacement>>,
    /// 複数巡目の目標列高さ
    #[serde(skip)]
    pub target_heights_list: Vec<Vec<i32>>,
}

fn default_sequence_bonus() -> f32 { 30.0 }
fn default_height_bonus() -> f32 { 5.0 }

// --------------------------------------------------------------------------
// ファイルロード
// --------------------------------------------------------------------------

pub fn load_opening(path: &str) -> std::io::Result<OpeningTemplate> {
    let file = std::fs::File::open(path)?;
    let reader = std::io::BufReader::new(file);
    let mut tmpl: OpeningTemplate = serde_json::from_reader(reader)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

    // 後方互換性: board_maps が無くて board_map がある場合、board_maps に格納
    if tmpl.board_maps.is_empty() && !tmpl.board_map.is_empty() {
        tmpl.board_maps = vec![tmpl.board_map.clone()];
    }

    // もし JSON 内に branches が定義されておらず、マップがある場合は
    // それを branches の最初の要素（条件なし）にする
    if tmpl.branches.is_empty() {
        if !tmpl.board_maps.is_empty() {
            tmpl.branches.push(OpeningBranch {
                conditions: Vec::new(),
                board_map: tmpl.board_map.clone(),
                board_maps: tmpl.board_maps.clone(),
                parsed_placements: Vec::new(),
                target_heights: Vec::new(),
                parsed_placements_list: Vec::new(),
                target_heights_list: Vec::new(),
            });
        }
    }

    // 各ブランチ内でも、board_maps が空で board_map がある場合は格納
    for branch in &mut tmpl.branches {
        if branch.board_maps.is_empty() && !branch.board_map.is_empty() {
            branch.board_maps = vec![branch.board_map.clone()];
        }
    }

    // 左右反転（ミラー）したブランチを自動生成して追加する
    let mut mirrored_branches = Vec::new();
    for branch in &tmpl.branches {
        let mir = create_mirror_branch(branch);
        mirrored_branches.push(mir);
    }
    tmpl.branches.extend(mirrored_branches);

    // すべてのブランチの board_maps をパース
    for branch in &mut tmpl.branches {
        branch.parsed_placements_list = Vec::new();
        branch.target_heights_list = Vec::new();

        for map in &branch.board_maps {
            if !map.is_empty() {
                let parsed = parse_board_map(map).unwrap_or_default();
                let heights = compute_target_heights(map);
                branch.parsed_placements_list.push(parsed);
                branch.target_heights_list.push(heights);
            } else {
                branch.parsed_placements_list.push(Vec::new());
                branch.target_heights_list.push(Vec::new());
            }
        }

        // 従来互換用
        if let Some(first_map) = branch.board_maps.first() {
            branch.board_map = first_map.clone();
            branch.parsed_placements = parse_board_map(first_map).unwrap_or_default();
            branch.target_heights = compute_target_heights(first_map);
        }
    }

    // 分岐なし互換用（ルートの直属フィールドの初期化）
    tmpl.parsed_placements_list = Vec::new();
    tmpl.target_heights_list = Vec::new();
    for map in &tmpl.board_maps {
        if !map.is_empty() {
            let parsed = parse_board_map(map).unwrap_or_default();
            let heights = compute_target_heights(map);
            tmpl.parsed_placements_list.push(parsed);
            tmpl.target_heights_list.push(heights);
        } else {
            tmpl.parsed_placements_list.push(Vec::new());
            tmpl.target_heights_list.push(Vec::new());
        }
    }

    if let Some(first_map) = tmpl.board_maps.first() {
        tmpl.board_map = first_map.clone();
        tmpl.parsed_placements = parse_board_map(first_map).unwrap_or_default();
        tmpl.target_heights = compute_target_heights(first_map);
    }

    Ok(tmpl)
}

fn create_mirror_branch(orig: &OpeningBranch) -> OpeningBranch {
    // 1. 各巡目の board_maps の反転と文字置換
    let mut mirrored_maps = Vec::new();
    for map in &orig.board_maps {
        let mut mir_map = Vec::new();
        for row in map {
            let rev_chars: String = row.chars().rev().map(|c| {
                match c.to_ascii_lowercase() {
                    'j' => 'l',
                    'l' => 'j',
                    's' => 'z',
                    'z' => 's',
                    other => other,
                }
            }).collect();
            mir_map.push(rev_chars);
        }
        mirrored_maps.push(mir_map);
    }

    // 2. 条件（conditions）の反転
    let mut mirrored_conditions = Vec::new();
    for cond in &orig.conditions {
        let mir_cond = mirror_condition(cond);
        mirrored_conditions.push(mir_cond);
    }

    OpeningBranch {
        conditions: mirrored_conditions,
        board_map: mirrored_maps.first().cloned().unwrap_or_default(),
        board_maps: mirrored_maps,
        parsed_placements: Vec::new(),
        target_heights: Vec::new(),
        parsed_placements_list: Vec::new(),
        target_heights_list: Vec::new(),
    }
}

fn mirror_condition(cond: &str) -> String {
    let swap_piece = |s: &str| -> String {
        match s.to_uppercase().as_str() {
            "J" => "L".to_string(),
            "L" => "J".to_string(),
            "S" => "Z".to_string(),
            "Z" => "S".to_string(),
            other => other.to_string(),
        }
    };

    if cond.contains('<') {
        let parts: Vec<&str> = cond.split('<').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let p1 = swap_piece(parts[0]);
            let p2 = swap_piece(parts[1]);
            return format!("{} < {}", p1, p2);
        }
    } else if cond.contains('=') {
        let parts: Vec<&str> = cond.split('=').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let p1 = swap_piece(parts[0]);
            let p2 = swap_piece(parts[1]);
            return format!("{} = {}", p1, p2);
        }
    }
    cond.to_string()
}


// --------------------------------------------------------------------------
// board_map パーサー
// --------------------------------------------------------------------------

/// board_map 文字列配列を解析して ParsedPlacement のリストを返す
fn parse_board_map(rows: &[String]) -> Option<Vec<ParsedPlacement>> {
    // 2D グリッドに変換（row, col）→ char
    let grid: Vec<Vec<char>> = rows
        .iter()
        .map(|r| r.to_lowercase().chars().collect())
        .collect();
    let nrows = grid.len();
    let ncols = if nrows > 0 { grid[0].len() } else { return None };

    // 各セルを未訪問としてマーク
    let mut visited = vec![vec![false; ncols]; nrows];
    let mut placements = Vec::new();

    for start_r in 0..nrows {
        for start_c in 0..ncols {
            let ch = grid[start_r][start_c];
            if ch == '0' || visited[start_r][start_c] {
                continue;
            }

            // BFS で連結成分を収集
            let mut region: Vec<(i32, i32)> = Vec::new();
            let mut queue: VecDeque<(usize, usize)> = VecDeque::new();
            queue.push_back((start_r, start_c));
            visited[start_r][start_c] = true;

            while let Some((r, c)) = queue.pop_front() {
                region.push((c as i32, r as i32)); // (x=col, y=row)
                for (dr, dc) in [(-1i32, 0), (1, 0), (0, -1i32), (0, 1)] {
                    let nr = r as i32 + dr;
                    let nc = c as i32 + dc;
                    if nr >= 0 && nr < nrows as i32 && nc >= 0 && nc < ncols as i32 {
                        let (nr, nc) = (nr as usize, nc as usize);
                        if !visited[nr][nc] && grid[nr][nc] == ch {
                            visited[nr][nc] = true;
                            queue.push_back((nr, nc));
                        }
                    }
                }
            }

            // ピース種の特定
            let block_type = match ch {
                'i' => BlockType::I,
                'o' => BlockType::O,
                't' => BlockType::T,
                's' => BlockType::S,
                'z' => BlockType::Z,
                'j' => BlockType::J,
                'l' => BlockType::L,
                _ => continue,
            };

            if region.len() != 4 {
                // 4マスでなければスキップ（不正な盤面）
                continue;
            }

            // 回転と列を特定
            if let Some((col, rotation)) = match_rotation(block_type, &region) {
                placements.push(ParsedPlacement { piece: block_type, col, rotation });
            }
        }
    }

    Some(placements)
}

/// 4マスのセル座標リストからミノの col と rotation を特定する
fn match_rotation(bt: BlockType, cells: &[(i32, i32)]) -> Option<(i32, usize)> {
    // 正規化: 最小 x, y を基準に相対座標へ
    let min_x = cells.iter().map(|(x, _)| *x).min()?;
    let min_y = cells.iter().map(|(_, y)| *y).min()?;
    let rel: HashSet<(i32, i32)> = cells.iter().map(|(x, y)| (x - min_x, y - min_y)).collect();

    // 各回転と比較
    let max_rot = if bt == BlockType::O { 1 } else { 4 };
    for rot in 0..max_rot {
        let offsets = get_piece_offsets(bt, rot);
        let min_ox = offsets.iter().map(|(dx, _)| *dx).min()?;
        let min_oy = offsets.iter().map(|(_, dy)| *dy).min()?;
        let norm: HashSet<(i32, i32)> = offsets.iter()
            .map(|(dx, dy)| (dx - min_ox, dy - min_oy))
            .collect();

        if rel == norm {
            // col = board_map での最左セルの x 座標
            // ただし get_piece_offsets の原点補正が必要
            // 配置 x = min_x - (-min_ox) = min_x + min_ox_from_origin
            // AI は piece.x を基準にオフセットを足すので:
            //   actual_min_x = piece.x + min_ox
            //   piece.x = min_x - min_ox
            let piece_x = min_x - min_ox;
            return Some((piece_x, rot));
        }
    }
    None
}

/// board_map から目標列高さを計算する（board_map の行数分が高さ）
fn compute_target_heights(rows: &[String]) -> Vec<i32> {
    let _nrows = rows.len() as i32;
    let ncols = rows.first().map_or(0, |r| r.len());
    let mut heights = vec![0i32; ncols];

    for c in 0..ncols {
        for (r, row) in rows.iter().enumerate() {
            let ch = row.to_lowercase().chars().nth(c).unwrap_or('0');
            if ch != '0' {
                // 上端から nrows 行分のうち、この列に何行あるか
                let col_height = rows.iter()
                    .filter(|row| {
                        row.to_lowercase().chars().nth(c).unwrap_or('0') != '0'
                    })
                    .count() as i32;
                heights[c] = col_height;
                break;
            }
            let _ = r;
        }
    }
    heights
}

// --------------------------------------------------------------------------
// AI 評価ヘルパー
// --------------------------------------------------------------------------

impl OpeningTemplate {
    pub fn get_active_branch(&self, game: &crate::tetris::Game) -> Option<&OpeningBranch> {
        if self.branches.is_empty() {
            return None;
        }

        // 1. まず、条件式が定義されており、かつ完全に一致するブランチをフィルタ
        let mut candidate_branches = Vec::new();
        for branch in &self.branches {
            if branch.conditions.is_empty() {
                continue;
            }
            let mut all_match = true;
            for cond in &branch.conditions {
                if !evaluate_condition(cond, &game.bag.queue, Some(game.current_piece.block_type)) {
                    all_match = false;
                    break;
                }
            }
            if all_match {
                candidate_branches.push(branch);
            }
        }

        // 条件一致ブランチがあれば、その中で「現在の盤面の高さに一番近いもの」を返す
        if !candidate_branches.is_empty() {
            return candidate_branches.into_iter()
                .max_by(|a, b| {
                    let fit_a = evaluate_branch_fit(&game.board, a);
                    let fit_b = evaluate_branch_fit(&game.board, b);
                    fit_a.partial_cmp(&fit_b).unwrap_or(std::cmp::Ordering::Equal)
                });
        }

        // 2. 条件一致ブランチがない場合、あるいは条件のないデフォルトブランチ群（ミラー含む）から、
        // 「現在の盤面の高さに一番近いもの」を返す
        self.branches.iter()
            .max_by(|a, b| {
                let fit_a = evaluate_branch_fit(&game.board, a);
                let fit_b = evaluate_branch_fit(&game.board, b);
                fit_a.partial_cmp(&fit_b).unwrap_or(std::cmp::Ordering::Equal)
            })
    }
}

fn evaluate_branch_fit(board: &Board, branch: &OpeningBranch) -> f32 {
    if branch.target_heights.is_empty() {
        return 0.0;
    }
    let mut heights = [0i32; BOARD_WIDTH];
    for x in 0..BOARD_WIDTH {
        for y in 0..INTERNAL_HEIGHT {
            if board[y][x].is_some() {
                heights[x] = (INTERNAL_HEIGHT - y) as i32;
                break;
            }
        }
    }
    let tol = 2.0f32;
    let mut bonus = 0.0f32;
    for x in 0..BOARD_WIDTH.min(branch.target_heights.len()) {
        let diff = (heights[x] - branch.target_heights[x]).abs() as f32;
        if diff <= tol {
            let closeness = 1.0 - diff / (tol + 1.0);
            bonus += closeness;
        }
    }
    bonus
}

fn evaluate_condition(cond: &str, queue: &[BlockType], current_piece: Option<BlockType>) -> bool {
    let mut sequence = Vec::new();
    if let Some(cp) = current_piece {
        sequence.push(cp);
    }
    sequence.extend_from_slice(queue);

    if cond.contains('<') {
        let parts: Vec<&str> = cond.split('<').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let p1 = parse_block_type_char(parts[0]);
            let p2 = parse_block_type_char(parts[1]);
            if let (Some(b1), Some(b2)) = (p1, p2) {
                let idx1 = sequence.iter().position(|&x| x == b1);
                let idx2 = sequence.iter().position(|&x| x == b2);
                match (idx1, idx2) {
                    (Some(i1), Some(i2)) => {
                        return i1 < i2;
                    }
                    _ => return false,
                }
            }
        }
    } else if cond.contains('=') {
        let parts: Vec<&str> = cond.split('=').map(|s| s.trim()).collect();
        if parts.len() == 2 {
            let p1 = parse_block_type_char(parts[0]);
            let p2 = parse_block_type_char(parts[1]);
            if let (Some(b1), Some(b2)) = (p1, p2) {
                // 等号（順不同）: 指定された両方のピースが現在の手またはネクストにあれば true
                let has1 = sequence.iter().any(|&x| x == b1);
                let has2 = sequence.iter().any(|&x| x == b2);
                return has1 && has2;
            }
        }
    }
    false
}

fn parse_block_type_char(s: &str) -> Option<BlockType> {
    match s.to_uppercase().as_str() {
        "I" => Some(BlockType::I),
        "O" => Some(BlockType::O),
        "T" => Some(BlockType::T),
        "S" => Some(BlockType::S),
        "Z" => Some(BlockType::Z),
        "J" => Some(BlockType::J),
        "L" => Some(BlockType::L),
        _ => None,
    }
}

/// 現在の盤面が board_map の目標高さにどれだけ近いかを評価する
pub fn evaluate_opening_fit(
    board: &Board,
    template: &OpeningTemplate,
    game: &crate::tetris::Game,
    opening_turn: usize,
) -> f32 {
    let bag_index = opening_turn / 7;

    let target_heights = if let Some(branch) = template.get_active_branch(game) {
        if bag_index < branch.target_heights_list.len() {
            &branch.target_heights_list[bag_index]
        } else {
            branch.target_heights_list.last().unwrap_or(&branch.target_heights)
        }
    } else {
        if bag_index < template.target_heights_list.len() {
            &template.target_heights_list[bag_index]
        } else {
            template.target_heights_list.last().unwrap_or(&template.target_heights)
        }
    };

    if target_heights.is_empty() {
        return 0.0;
    }

    let mut heights = [0i32; BOARD_WIDTH];
    for x in 0..BOARD_WIDTH {
        for y in 0..INTERNAL_HEIGHT {
            if board[y][x].is_some() {
                heights[x] = (INTERNAL_HEIGHT - y) as i32;
                break;
            }
        }
    }

    let tol = 2.0f32;
    let mut bonus = 0.0f32;

    for x in 0..BOARD_WIDTH.min(target_heights.len()) {
        let diff = (heights[x] - target_heights[x]).abs() as f32;
        if diff <= tol {
            let closeness = 1.0 - diff / (tol + 1.0);
            bonus += template.opening_bonus_per_column * closeness;
        }
    }

    bonus
}

/// 候補手が board_map の配置指示と一致するかチェックしてボーナスを返す
pub fn evaluate_sequence_match(
    template: &OpeningTemplate,
    game: &crate::tetris::Game,
    turn_index: usize,
    current_piece: BlockType,
    candidate_col: i32,
    candidate_rotation: usize,
) -> f32 {
    let bag_index = turn_index / 7;

    let placements = if let Some(branch) = template.get_active_branch(game) {
        if bag_index < branch.parsed_placements_list.len() {
            &branch.parsed_placements_list[bag_index]
        } else {
            branch.parsed_placements_list.last().unwrap_or(&branch.parsed_placements)
        }
    } else {
        if bag_index < template.parsed_placements_list.len() {
            &template.parsed_placements_list[bag_index]
        } else {
            template.parsed_placements_list.last().unwrap_or(&template.parsed_placements)
        }
    };

    for placement in placements {
        if placement.piece != current_piece {
            continue;
        }
        let col_ok = (candidate_col - placement.col).abs() <= 1;
        let rot_ok = candidate_rotation == placement.rotation;

        if col_ok && rot_ok {
            return template.sequence_match_bonus;
        } else if col_ok || rot_ok {
            return template.sequence_match_bonus * 0.3;
        }
    }
    0.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_openings() {
        // 63積みのロードテスト
        let stacking = load_opening("templates/openings/63_stacking.json").unwrap();
        assert_eq!(stacking.name, "63積み");
        assert!(!stacking.parsed_placements.is_empty());
        // 7つのミノがパースされるはず
        assert_eq!(stacking.parsed_placements.len(), 7);

        // TSDオープナーのロードテスト
        let tsd = load_opening("templates/openings/tsd_opener.json").unwrap();
        assert_eq!(tsd.name, "TSD オープナー");
        // オリジナルとミラーの2つが branches にあるはず
        assert_eq!(tsd.branches.len(), 2);
        assert_eq!(tsd.branches[0].board_map[0], "00s00z0t00");
        assert_eq!(tsd.branches[0].board_map[3], "iiii0jjjll");
        assert_eq!(tsd.branches[1].board_map[0], "00t0s00z00");
        assert_eq!(tsd.branches[1].board_map[3], "jjlll0iiii");
        assert_eq!(tsd.parsed_placements.len(), 7);

        // C4Wのロードテスト
        let c4w = load_opening("templates/openings/c4w_ren.json").unwrap();
        assert_eq!(c4w.name, "C4W 中開けREN");
        // 左側の I, O, L、右側の I, O, J の合計6つのミノがパースされるはず
        assert_eq!(c4w.parsed_placements.len(), 6);

        // マルチバッグのロードテスト
        let mb = load_opening("templates/openings/test_multibag.json").unwrap();
        assert_eq!(mb.name, "マルチバッグテスト");
        assert_eq!(mb.board_maps.len(), 2);
        assert_eq!(mb.parsed_placements_list.len(), 2);
        assert_eq!(mb.parsed_placements_list[0].len(), 2); // 1st Bag has I, O
        assert_eq!(mb.parsed_placements_list[1].len(), 1); // 2nd Bag has S
    }

    #[test]
    fn test_evaluate_condition() {
        // キューに [J, I, S, Z, O, L] があり、現在が T ミノの場合
        // 全体の出現順序: T -> J -> I -> S -> Z -> O -> L
        let queue = vec![BlockType::J, BlockType::I, BlockType::S, BlockType::Z, BlockType::O, BlockType::L];
        let current = Some(BlockType::T);

        // T < J は true (Tは0, Jは1)
        assert!(evaluate_condition("T < J", &queue, current));
        // J < L は true (Jは1, Lは6)
        assert!(evaluate_condition("J < L", &queue, current));
        // L < J は false
        assert!(!evaluate_condition("L < J", &queue, current));
        // O < Z は false (Zは4, Oは5)
        assert!(!evaluate_condition("O < Z", &queue, current));
        // Z < O は true
        assert!(evaluate_condition("Z < O", &queue, current));

        // 等号（順不同）のテスト
        assert!(evaluate_condition("J = L", &queue, current));
        assert!(evaluate_condition("T = Z", &queue, current));
        // キューにないピースの場合は false になるはず
        let queue_no_l = vec![BlockType::J, BlockType::I, BlockType::S, BlockType::Z, BlockType::O];
        assert!(!evaluate_condition("J = L", &queue_no_l, current));
    }
}
