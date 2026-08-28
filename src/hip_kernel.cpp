#include <hip/hip_runtime.h>
#include <cmath>
#include <cstdio>

extern "C" {

struct HipEvaluatorContext {
    float* d_weights;
    float* d_features;
    float* d_scores;
    int max_candidates;
    int max_features;
    bool initialized;
};

static HipEvaluatorContext g_hip_ctx = { nullptr, nullptr, nullptr, 0, 0, false };

__global__ void evaluate_linear_kernel(
    const float* __restrict__ weights,
    const float* __restrict__ features,
    float* __restrict__ scores,
    int num_candidates,
    int num_features
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < num_candidates) {
        float score = 0.0f;
        int offset = idx * num_features;
        for (int i = 0; i < num_features; ++i) {
            score += weights[i] * features[offset + i];
        }
        scores[idx] = score;
    }
}

// 20-Feature Non-Linear Hybrid Polynomial Interaction Kernel (addplan.md Section 6-16)
__global__ void evaluate_nonlinear_kernel(
    const float* __restrict__ weights,
    const float* __restrict__ features,
    float* __restrict__ scores,
    int num_candidates,
    int num_features
) {
    int idx = blockIdx.x * blockDim.x + threadIdx.x;
    if (idx < num_candidates) {
        int offset = idx * num_features;
        
        // 1. 一次項 (Linear terms): Σ wi * xi
        float score = 0.0f;
        for (int i = 0; i < num_features; ++i) {
            score += weights[i] * features[offset + i];
        }

        if (num_features >= 20) {
            // 正規化特徴量: x0..x19 (addplan.md Section 4)
            float x_tspin        = features[offset + 0];
            float x_tspin_trn    = features[offset + 1];
            float x_hole         = features[offset + 2];
            float x_hole_spread  = features[offset + 3];
            float x_placement    = features[offset + 4];
            float x_tetris       = features[offset + 5];
            float x_ren          = features[offset + 9];
            float x_btb          = features[offset + 10];
            float x_combo        = features[offset + 11];
            float x_pc           = features[offset + 13];
            float x_height       = features[offset + 14];
            float x_max_height   = features[offset + 15];
            float x_bumpiness    = features[offset + 16];
            float x_well_quality = features[offset + 17];
            float x_overhang     = features[offset + 18];
            float x_future_fit   = features[offset + 19];

            // 2. 二次交互作用項 (Quadratic interactions: aij * xi * xj - pi * xi^2)
            score += 25.0f * (x_tspin * x_tspin_trn);
            score += 30.0f * (x_tetris * x_well_quality);
            score += 20.0f * (x_tetris * x_btb);
            score += 15.0f * (x_placement * x_future_fit);
            score -= 15.0f * (x_hole * x_hole_spread);
            score -= 20.0f * (x_max_height * x_hole);
            score -= 10.0f * (x_overhang * x_hole);
            score -= 5.0f * (x_height * x_bumpiness);

            // 過剰評価抑制項 (- pi * xi^2)
            score -= 8.0f * (x_bumpiness * x_bumpiness);
            score -= 12.0f * (x_hole * x_hole);

            // 3. 三次項 (Cubic interactions: bijk * xi * xj * xk)
            score += 40.0f * (x_tetris * x_well_quality * x_btb);
            score += 35.0f * (x_tspin * x_tspin_trn * x_future_fit);
            score += 20.0f * (x_ren * x_combo * x_future_fit);
            score -= 25.0f * (x_hole * x_hole_spread * x_max_height);

            // 4. 非線形ペナルティ & 飽和型ボーナス (addplan.md Section 9-11)
            // 指数型高さペナルティ: D_height = exp(3.0 * (MaxHeight - 0.6))
            if (x_max_height > 0.6f) {
                score -= 60.0f * (expf(3.0f * (x_max_height - 0.6f)) - 1.0f);
            }
            // 累乗型穴ペナルティ: H + 1.5*H^2 + 2.0*H^3
            score -= 30.0f * (x_hole + 1.5f * x_hole * x_hole + 2.0f * x_hole * x_hole * x_hole);
            // ガウス型井戸最適品質ボーナス: A * exp(- (Well - 0.85)^2 / 0.08)
            float well_diff = x_well_quality - 0.85f;
            score += 25.0f * expf(-(well_diff * well_diff) / 0.08f);
            // REN / BTB / Combo 飽和型ボーナス: A * (1 - exp(-k * x))
            score += 30.0f * (1.0f - expf(-1.5f * x_ren));
            score += 25.0f * (1.0f - expf(-1.2f * x_btb));
            score += 20.0f * (1.0f - expf(-1.0f * x_combo));
            // PCボーナス: シグモイド型
            score += 100.0f * (1.0f / (1.0f + expf(-5.0f * (x_pc - 0.5f))));
        }

        scores[idx] = score;
    }
}

int hip_evaluator_init(int max_candidates, int max_features) {
    int count = 0;
    if (hipGetDeviceCount(&count) != hipSuccess || count == 0) {
        return -1;
    }

    g_hip_ctx.max_candidates = max_candidates;
    g_hip_ctx.max_features = max_features;

    size_t weights_size = (size_t)max_features * sizeof(float);
    size_t features_size = (size_t)max_candidates * max_features * sizeof(float);
    size_t scores_size = (size_t)max_candidates * sizeof(float);

    if (hipMalloc(&g_hip_ctx.d_weights, weights_size) != hipSuccess) return -2;
    if (hipMalloc(&g_hip_ctx.d_features, features_size) != hipSuccess) return -3;
    if (hipMalloc(&g_hip_ctx.d_scores, scores_size) != hipSuccess) return -4;

    g_hip_ctx.initialized = true;
    return 0;
}

int hip_evaluator_evaluate_batch(
    const float* h_weights,
    const float* h_features,
    float* h_scores,
    int num_candidates,
    int num_features,
    int is_nonlinear
) {
    if (!g_hip_ctx.initialized || num_candidates > g_hip_ctx.max_candidates || num_features > g_hip_ctx.max_features) {
        return -1;
    }

    size_t weights_size = (size_t)num_features * sizeof(float);
    size_t features_size = (size_t)num_candidates * num_features * sizeof(float);
    size_t scores_size = (size_t)num_candidates * sizeof(float);

    hipMemcpy(g_hip_ctx.d_weights, h_weights, weights_size, hipMemcpyHostToDevice);
    hipMemcpy(g_hip_ctx.d_features, h_features, features_size, hipMemcpyHostToDevice);

    int block_size = 256;
    int grid_size = (num_candidates + block_size - 1) / block_size;

    if (is_nonlinear) {
        evaluate_nonlinear_kernel<<<grid_size, block_size>>>(
            g_hip_ctx.d_weights, g_hip_ctx.d_features, g_hip_ctx.d_scores, num_candidates, num_features
        );
    } else {
        evaluate_linear_kernel<<<grid_size, block_size>>>(
            g_hip_ctx.d_weights, g_hip_ctx.d_features, g_hip_ctx.d_scores, num_candidates, num_features
        );
    }

    hipDeviceSynchronize();
    hipMemcpy(h_scores, g_hip_ctx.d_scores, scores_size, hipMemcpyDeviceToHost);

    return 0;
}

void hip_evaluator_cleanup() {
    if (g_hip_ctx.initialized) {
        if (g_hip_ctx.d_weights) hipFree(g_hip_ctx.d_weights);
        if (g_hip_ctx.d_features) hipFree(g_hip_ctx.d_features);
        if (g_hip_ctx.d_scores) hipFree(g_hip_ctx.d_scores);
        g_hip_ctx.d_weights = nullptr;
        g_hip_ctx.d_features = nullptr;
        g_hip_ctx.d_scores = nullptr;
        g_hip_ctx.initialized = false;
    }
}

} // extern "C"
