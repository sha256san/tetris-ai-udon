# 04. 火力・効率・APM・Spikeダメージ理論 (Firepower Theory)

## 1. 火力指標の定義
- **APM (Attack Per Minute)**: 1分間に対戦相手へ送信したガーベージ段数。
- **APL (Attack Per Line)**: 消去したライン1行あたりに生成された攻撃段数。
  - Tetris (4ライン消去 / 4段攻撃) $\rightarrow$ **1.00 APL**
  - T-Spin Double (2ライン消去 / 4段攻撃) $\rightarrow$ **2.00 APL** (Tetrisの2倍効率)
  - T-Spin Triple (3ライン消去 / 6段攻撃) $\rightarrow$ **2.00 APL**
  - Back-to-Back TSD (2ライン消去 / 5段攻撃) $\rightarrow$ **2.50 APL** (最強の持続効率)
- **APP (Attack Per Piece)**: ミノ1個消費あたりに生み出す攻撃段数。

## 2. Spike ダメージ vs Sustained (持続) 火力
- **Spike (瞬間集中火力)**: 
  - 相手が相殺できない短時間（1〜3秒以内）に10〜20段以上のガーベージを一気に送り込む戦術。
  - 例: DT砲（TST $\rightarrow$ TSD: 計11段）や、B2B TSD 3連打。
- **Sustained (持続火力)**:
  - 毎分80〜120 APMを安定して維持し、相手のリソースを枯渇させる戦術。
  - LST積み、平積みドネイト、STSD連打。

## 3. 高火力地形の要件
1. **中央山型を作らない平坦な土台**: どの列にも柔軟にミノを置ける広いスペース。
2. **2〜9列目（3〜8列目推奨）の単一列穴**: TSD発火後も即座に次の攻撃へ移行可能。
3. **両端同時空きゼロ**: Iミノを無駄遣いせずテトリスやドネイトに温存。
