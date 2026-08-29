# 10-C. 状態表現・盤面エンコーディング (State Representation)

## 1. 盤面データ構造
- `Board = [[Option<BlockType>; 10]; 24]`
- 0〜19行: 可視領域、20〜23行: バッファ・出現領域。

## 2. 候補手データ構造 (`CandidateMove`)
- `x: i32, rotation: usize, use_hold: bool`
- `features: Vec<f32>` (20次元)
- `eval_score: f32`
- `was_rotate: bool`
- `path: Vec<MoveAction>`
