# 探索アルゴリズム & GPUバックエンド ベンチマーク検証レポート

- **実施日時**: 2026-08-28
- **ROCm Compute**: AMD Radeon RX 9060 XT (ROCm 7.1 HIP / gfx1200)
- **Vulkan Compute**: GPU (Discrete): AMD Radeon RX 9060 XT (RADV GFX1200) (Vulkan)
- **共通検証シード数**: 5 シード (固定シード公平比較)
- **1ゲーム最大ミノ数**: 400 ミノ

---

## 1. 総合ベンチマーク結果ランキング

| 順位 | アルゴリズム & バックエンド構成 | 先読み深さ | ビーム幅 | 平均消去ライン | 平均スコア | 平均PPS | 探索速度 (ms/手) | 総合スコア |
|---|---|---|---|---|---|---|---|---|
| **🥇 1位** | **6. Base 1-Ply (No Lookahead) [ROCm HIP]** | Depth 1 | Width 1 | **61.8** | **9974** | 2218.7 | 0.45 ms | **4509.5** |
| **🥈 2位** | **3. Beam Search (Depth 3, Width 50) [CPU Multi-thread]** | Depth 3 | Width 50 | **47.2** | **44925** | 66.8 | 15.05 ms | **158.2** |
| **🥉 3位** | **5. Beam Search (Depth 5, Width 30) [Vulkan wgpu]** | Depth 5 | Width 30 | **154.4** | **116532** | 19.4 | 51.56 ms | **116.9** |
| **4位** | **4. Beam Search (Depth 5, Width 30) [ROCm HIP]** | Depth 5 | Width 30 | **154.4** | **116532** | 14.5 | 69.13 ms | **106.8** |
| **5位** | **2. Beam Search (Depth 3, Width 50) [Vulkan wgpu]** | Depth 3 | Width 50 | **128.8** | **88006** | 20.5 | 48.74 ms | **106.2** |
| **6位** | **1. Beam Search (Depth 3, Width 50) [ROCm HIP]** | Depth 3 | Width 50 | **128.8** | **88006** | 15.3 | 65.29 ms | **95.6** |

---

## 2. 最適構成の分析と結論

### ★ 最優秀構成: **6. Base 1-Ply (No Lookahead) [ROCm HIP]**

- **平均消去ライン数**: 61.8 ライン (最大: 133 ライン)
- **平均スコア**: 9974 点
- **1手あたり探索時間**: 0.45 ms (2218.7 PPS)
- **詳細説明**: 単手評価のみ（先読みなしのベースライン）

