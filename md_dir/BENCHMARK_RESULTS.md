# 探索アルゴリズム & GPUバックエンド ベンチマーク検証レポート

- **実施日時**: 2026-08-29
- **ROCm Compute**: AMD Radeon RX 9060 XT (ROCm 7.1 HIP / gfx1200)
- **Vulkan Compute**: GPU (Discrete): AMD Radeon RX 9060 XT (RADV GFX1200) (Vulkan)
- **共通検証シード数**: 5 シード (固定シード公平比較)
- **1ゲーム最大ミノ数**: 400 ミノ

---

## 1. 総合ベンチマーク結果ランキング

| 順位 | アルゴリズム & バックエンド構成 | 先読み深さ | ビーム幅 | 平均消去ライン | 平均スコア | 平均PPS | 探索速度 (ms/手) | 総合スコア |
|---|---|---|---|---|---|---|---|---|
| **🥇 1位** | **6. Base 1-Ply (No Lookahead) [ROCm HIP]** | Depth 1 | Width 1 | **14.0** | **5439** | 2177.4 | 0.46 ms | **4371.0** |
| **🥈 2位** | **3. Beam Search (Depth 3, Width 50) [CPU Multi-thread]** | Depth 3 | Width 50 | **10.4** | **5593** | 68.5 | 14.71 ms | **142.3** |
| **🥉 3位** | **5. Beam Search (Depth 5, Width 30) [Vulkan wgpu]** | Depth 5 | Width 30 | **61.2** | **24958** | 20.0 | 50.12 ms | **70.9** |
| **4位** | **4. Beam Search (Depth 5, Width 30) [ROCm HIP]** | Depth 5 | Width 30 | **61.2** | **24958** | 14.9 | 67.23 ms | **60.6** |
| **5位** | **2. Beam Search (Depth 3, Width 50) [Vulkan wgpu]** | Depth 3 | Width 50 | **32.0** | **11875** | 20.7 | 48.26 ms | **57.7** |
| **6位** | **1. Beam Search (Depth 3, Width 50) [ROCm HIP]** | Depth 3 | Width 50 | **32.0** | **11875** | 15.3 | 65.31 ms | **46.8** |

---

## 2. T-Spin 内訳詳細分析表 (T-Spin Breakdown by Category)

| アルゴリズム構成 | T-Spin Single (TSS) | T-Spin Double (TSD) | T-Spin Triple (TST) | T-Spin Mini | T-Spin 総計 | T-Slot 形成回数 | Tetris (4列消去) |
|---|---|---|---|---|---|---|---|
| **6. Base 1-Ply (No Lookahead) [ROCm HIP]** | **2.80 回** | **0.60 回** | **0.00 回** | **6.00 回** | **9.40 回** | **142.4 回** | **0.0 回** |
| **3. Beam Search (Depth 3, Width 50) [CPU Multi-thread]** | **0.80 回** | **1.20 回** | **0.00 回** | **7.60 回** | **9.60 回** | **206.0 回** | **0.2 回** |
| **5. Beam Search (Depth 5, Width 30) [Vulkan wgpu]** | **5.00 回** | **5.00 回** | **0.00 回** | **14.40 回** | **24.40 回** | **261.6 回** | **3.8 回** |
| **4. Beam Search (Depth 5, Width 30) [ROCm HIP]** | **5.00 回** | **5.00 回** | **0.00 回** | **14.40 回** | **24.40 回** | **261.6 回** | **3.8 回** |
| **2. Beam Search (Depth 3, Width 50) [Vulkan wgpu]** | **3.00 回** | **3.80 回** | **0.00 回** | **7.60 回** | **14.40 回** | **171.4 回** | **0.8 回** |
| **1. Beam Search (Depth 3, Width 50) [ROCm HIP]** | **3.00 回** | **3.80 回** | **0.00 回** | **7.60 回** | **14.40 回** | **171.4 回** | **0.8 回** |

---

## 3. 最適構成の分析と結論

### ★ 最優秀構成: **6. Base 1-Ply (No Lookahead) [ROCm HIP]**

- **平均消去ライン数**: 14.0 ライン (最大: 20 ライン)
- **平均スコア**: 5439 点
- **平均 T-Spin 回数**: 9.40 回 (TSD: 0.60回, TST: 0.00回, TSS: 2.80回, Mini: 6.00回)
- **平均 T-Slot 構築回数**: 142.4 回
- **1手あたり探索時間**: 0.46 ms (2177.4 PPS)
- **詳細説明**: 単手評価のみ（先読みなしのベースライン）

