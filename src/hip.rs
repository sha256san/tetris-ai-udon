use std::sync::{OnceLock, Mutex};

#[derive(Debug, Clone)]
pub struct HipEvaluator {
    pub is_available: bool,
    pub device_name: String,
}

unsafe extern "C" {
    fn hip_evaluator_init(max_candidates: i32, max_features: i32) -> i32;
    fn hip_evaluator_evaluate_batch(
        weights: *const f32,
        features: *const f32,
        scores: *mut f32,
        num_candidates: i32,
        num_features: i32,
        is_nonlinear: i32,
    ) -> i32;
    #[allow(dead_code)]
    fn hip_evaluator_cleanup();
}

static HIP_EVALUATOR: OnceLock<HipEvaluator> = OnceLock::new();
static HIP_MUTEX: Mutex<()> = Mutex::new(());

pub fn get_hip_evaluator() -> &'static HipEvaluator {
    HIP_EVALUATOR.get_or_init(|| {
        let max_candidates = 8192;
        let max_features = 32;
        let res = unsafe { hip_evaluator_init(max_candidates, max_features) };
        if res == 0 {
            HipEvaluator {
                is_available: true,
                device_name: "AMD Radeon RX 9060 XT (ROCm 7.1 HIP / gfx1200)".to_string(),
            }
        } else {
            HipEvaluator {
                is_available: false,
                device_name: "ROCm HIP Not Available".to_string(),
            }
        }
    })
}

impl HipEvaluator {
    pub fn evaluate_batch(&self, weights: &[f32], feature_batch: &[Vec<f32>], is_nonlinear: bool) -> Option<Vec<f32>> {
        if !self.is_available || feature_batch.is_empty() {
            return None;
        }

        let num_candidates = feature_batch.len();
        let num_features = weights.len();
        let mut flattened_features = Vec::with_capacity(num_candidates * num_features);
        for feats in feature_batch {
            for i in 0..num_features {
                flattened_features.push(feats.get(i).cloned().unwrap_or(0.0));
            }
        }

        let mut scores = vec![0.0f32; num_candidates];

        let _guard = match HIP_MUTEX.lock() {
            Ok(g) => g,
            Err(_) => return None,
        };

        let res = unsafe {
            hip_evaluator_evaluate_batch(
                weights.as_ptr(),
                flattened_features.as_ptr(),
                scores.as_mut_ptr(),
                num_candidates as i32,
                num_features as i32,
                if is_nonlinear { 1 } else { 0 },
            )
        };

        if res == 0 {
            Some(scores)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hip_evaluator_linear_and_nonlinear() {
        let evaluator = get_hip_evaluator();
        println!("HIP Device: {}", evaluator.device_name);
        if evaluator.is_available {
            let weights = vec![1.0, 2.0, -1.0];
            let features = vec![
                vec![1.0, 0.5, 2.0],
                vec![2.0, 1.0, 0.0],
            ];
            let scores = evaluator.evaluate_batch(&weights, &features, false);
            assert!(scores.is_some());
            let s = scores.unwrap();
            assert_eq!(s.len(), 2);
            assert!((s[0] - 0.0).abs() < 1e-4);
            assert!((s[1] - 4.0).abs() < 1e-4);

            // Test 20-feature non-linear
            let weights20 = vec![1.0; 20];
            let features20 = vec![vec![0.5; 20]];
            let scores20 = evaluator.evaluate_batch(&weights20, &features20, true);
            assert!(scores20.is_some());
        }
    }
}
