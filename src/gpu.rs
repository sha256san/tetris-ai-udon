use std::sync::{OnceLock, Mutex};

#[derive(Debug, Clone)]
pub enum GpuBackendType {
    DiscreteGpu(String),
    IntegratedGpu(String),
    CpuFallback(String),
}

struct GpuContext {
    device: wgpu::Device,
    queue: wgpu::Queue,
    pipeline: wgpu::ComputePipeline,
    bind_group: wgpu::BindGroup,
    params_buffer: wgpu::Buffer,
    weights_buffer: wgpu::Buffer,
    features_buffer: wgpu::Buffer,
    scores_buffer: wgpu::Buffer,
    readback_buffer: wgpu::Buffer,
}

pub struct GpuEvaluator {
    pub backend_info: GpuBackendType,
    context: Option<Mutex<GpuContext>>,
    max_batch_size: usize,
}

static GPU_EVALUATOR: OnceLock<GpuEvaluator> = OnceLock::new();

pub fn get_gpu_evaluator() -> &'static GpuEvaluator {
    GPU_EVALUATOR.get_or_init(|| GpuEvaluator::new())
}

impl GpuEvaluator {
    pub fn new() -> Self {
        let future = async {
            let instance = wgpu::Instance::default();

            // 1. 独立型GPU (Discrete GPU: NVIDIA/AMD Radeon等) を優先探索
            let mut adapter_opt = instance
                .request_adapter(&wgpu::RequestAdapterOptions {
                    power_preference: wgpu::PowerPreference::HighPerformance,
                    force_fallback_adapter: false,
                    compatible_surface: None,
                })
                .await;

            let mut is_discrete = true;

            // 2. 独立型GPUがない場合、内蔵グラフィック (Integrated GPU) を探索
            if adapter_opt.is_none() {
                adapter_opt = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::LowPower,
                        force_fallback_adapter: false,
                        compatible_surface: None,
                    })
                    .await;
                is_discrete = false;
            }

            // 3. フォールバックアダプタ（ソフトウェアレンダラー/CPU Vulkan）の探索
            if adapter_opt.is_none() {
                adapter_opt = instance
                    .request_adapter(&wgpu::RequestAdapterOptions {
                        power_preference: wgpu::PowerPreference::None,
                        force_fallback_adapter: true,
                        compatible_surface: None,
                    })
                    .await;
                is_discrete = false;
            }

            let adapter = match adapter_opt {
                Some(a) => a,
                None => {
                    return GpuEvaluator {
                        backend_info: GpuBackendType::CpuFallback("No GPU Adapter Found (Rayon CPU)".into()),
                        context: None,
                        max_batch_size: 0,
                    };
                }
            };

            let info = adapter.get_info();
            let gpu_name = format!("{} ({:?})", info.name, info.backend);

            let backend_info = if is_discrete && info.device_type == wgpu::DeviceType::DiscreteGpu {
                GpuBackendType::DiscreteGpu(gpu_name)
            } else if info.device_type == wgpu::DeviceType::IntegratedGpu || !is_discrete {
                GpuBackendType::IntegratedGpu(gpu_name)
            } else {
                GpuBackendType::CpuFallback(format!("CPU Software Adapter: {}", info.name))
            };

            let (device, queue) = match adapter
                .request_device(
                    &wgpu::DeviceDescriptor {
                        label: Some("Tetris AI GPU Device"),
                        required_features: wgpu::Features::empty(),
                        required_limits: wgpu::Limits::downlevel_defaults(),
                        memory_hints: wgpu::MemoryHints::Performance,
                    },
                    None,
                )
                .await
            {
                Ok(res) => res,
                Err(e) => {
                    return GpuEvaluator {
                        backend_info: GpuBackendType::CpuFallback(format!("Device Init Failed: {} (Rayon CPU)", e)),
                        context: None,
                        max_batch_size: 0,
                    };
                }
            };

            // WGSL Compute Shader: 大規模候補手の特徴量評価を並列計算 (addplan.md 20特徴量非線形多項式対応)
            let shader_source = r#"
                struct Params {
                    num_candidates: u32,
                    num_features: u32,
                    is_nonlinear: u32,
                    padding: u32,
                };

                @group(0) @binding(0) var<uniform> params: Params;
                @group(0) @binding(1) var<storage, read> weights: array<f32>;
                @group(0) @binding(2) var<storage, read> features: array<f32>;
                @group(0) @binding(3) var<storage, read_write> scores: array<f32>;

                @compute @workgroup_size(64)
                fn main(@builtin(global_invocation_id) global_id: vec3<u32>) {
                    let index = global_id.x;
                    if (index >= params.num_candidates) {
                        return;
                    }

                    var score: f32 = 0.0;
                    let feature_offset = index * params.num_features;
                    for (var i: u32 = 0u; i < params.num_features; i = i + 1u) {
                        score += weights[i] * features[feature_offset + i];
                    }

                    if (params.is_nonlinear == 1u && params.num_features >= 20u) {
                        let x_tspin        = features[feature_offset + 0u];
                        let x_tspin_trn    = features[feature_offset + 1u];
                        let x_hole         = features[feature_offset + 2u];
                        let x_hole_spread  = features[feature_offset + 3u];
                        let x_placement    = features[feature_offset + 4u];
                        let x_tetris       = features[feature_offset + 5u];
                        let x_ren          = features[feature_offset + 9u];
                        let x_btb          = features[feature_offset + 10u];
                        let x_combo        = features[feature_offset + 11u];
                        let x_pc           = features[feature_offset + 13u];
                        let x_height       = features[feature_offset + 14u];
                        let x_max_height   = features[feature_offset + 15u];
                        let x_bumpiness    = features[feature_offset + 16u];
                        let x_well_quality = features[feature_offset + 17u];
                        let x_overhang     = features[feature_offset + 18u];
                        let x_future_fit   = features[feature_offset + 19u];

                        // 二次交互作用項 (Quadratic interactions)
                        score += 75.0 * (x_tspin * x_tspin_trn);
                        score += 50.0 * (x_tspin * x_btb);
                        score += 35.0 * (x_tspin_trn * x_future_fit);
                        score += 30.0 * (x_tetris * x_well_quality);
                        score += 20.0 * (x_tetris * x_btb);
                        score += 15.0 * (x_placement * x_future_fit);
                        score -= 15.0 * (x_hole * x_hole_spread);
                        score -= 20.0 * (x_max_height * x_hole);
                        score -= 10.0 * (x_overhang * x_hole);
                        score -= 5.0 * (x_height * x_bumpiness);
                        score -= 8.0 * (x_bumpiness * x_bumpiness);
                        score -= 12.0 * (x_hole * x_hole);

                        // 三次項 (Cubic interactions)
                        score += 90.0 * (x_tspin * x_tspin_trn * x_future_fit);
                        score += 60.0 * (x_tspin * x_tspin_trn * x_btb);
                        score += 40.0 * (x_tetris * x_well_quality * x_btb);
                        score += 20.0 * (x_ren * x_combo * x_future_fit);
                        score -= 25.0 * (x_hole * x_hole_spread * x_max_height);

                        // 非線形ペナルティ & 飽和型ボーナス
                        if (x_max_height > 0.6) {
                            score -= 60.0 * (exp(3.0 * (x_max_height - 0.6)) - 1.0);
                        }
                        score -= 30.0 * (x_hole + 1.5 * x_hole * x_hole + 2.0 * x_hole * x_hole * x_hole);
                        let well_diff = x_well_quality - 0.85;
                        score += 25.0 * exp(-(well_diff * well_diff) / 0.08);
                        score += 30.0 * (1.0 - exp(-1.5 * x_ren));
                        score += 25.0 * (1.0 - exp(-1.2 * x_btb));
                        score += 20.0 * (1.0 - exp(-1.0 * x_combo));
                        score += 100.0 * (1.0 / (1.0 + exp(-5.0 * (x_pc - 0.5))));
                    }

                    scores[index] = score;
                }
            "#;

            let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
                label: Some("Tetris Evaluator Shader"),
                source: wgpu::ShaderSource::Wgsl(shader_source.into()),
            });

            let bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("Evaluator Bind Group Layout"),
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 2,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 3,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
            });

            let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Evaluator Pipeline Layout"),
                bind_group_layouts: &[&bind_group_layout],
                push_constant_ranges: &[],
            });

            let pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                label: Some("Evaluator Compute Pipeline"),
                layout: Some(&pipeline_layout),
                module: &shader,
                entry_point: Some("main"),
                compilation_options: Default::default(),
                cache: None,
            });

            let max_batch_size = 8192;
            let max_features = 32;

            let params_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Persistent Params Buffer"),
                size: 16,
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let weights_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Persistent Weights Buffer"),
                size: (max_features * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let features_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Persistent Features Buffer"),
                size: (max_batch_size * max_features * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let scores_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Persistent Scores Buffer"),
                size: (max_batch_size * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_SRC,
                mapped_at_creation: false,
            });

            let readback_buffer = device.create_buffer(&wgpu::BufferDescriptor {
                label: Some("Persistent Readback Buffer"),
                size: (max_batch_size * std::mem::size_of::<f32>()) as u64,
                usage: wgpu::BufferUsages::MAP_READ | wgpu::BufferUsages::COPY_DST,
                mapped_at_creation: false,
            });

            let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label: Some("Persistent Evaluator Bind Group"),
                layout: &bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: params_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: weights_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: features_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 3,
                        resource: scores_buffer.as_entire_binding(),
                    },
                ],
            });

            let context = GpuContext {
                device,
                queue,
                pipeline,
                bind_group,
                params_buffer,
                weights_buffer,
                features_buffer,
                scores_buffer,
                readback_buffer,
            };

            GpuEvaluator {
                backend_info,
                context: Some(Mutex::new(context)),
                max_batch_size,
            }
        };

        futures::executor::block_on(future)
    }

    pub fn is_gpu_available(&self) -> bool {
        self.context.is_some()
    }

    pub fn get_info_string(&self) -> String {
        match &self.backend_info {
            GpuBackendType::DiscreteGpu(name) => format!("GPU (Discrete): {}", name),
            GpuBackendType::IntegratedGpu(name) => format!("GPU (Integrated): {}", name),
            GpuBackendType::CpuFallback(msg) => format!("CPU Fallback: {}", msg),
        }
    }

    pub fn evaluate_batch(&self, weights: &[f32], feature_batch: &[Vec<f32>], is_nonlinear: bool) -> Vec<f32> {
        let num_candidates = feature_batch.len();
        if num_candidates == 0 {
            return Vec::new();
        }

        let num_features = weights.len();

        let cpu_fallback = || -> Vec<f32> {
            feature_batch
                .iter()
                .map(|feats| {
                    let mut s = 0.0f32;
                    for i in 0..num_features.min(feats.len()) {
                        s += weights[i] * feats[i];
                    }
                    s
                })
                .collect()
        };

        if !self.is_gpu_available() || num_candidates > self.max_batch_size {
            return cpu_fallback();
        }

        let context_lock = match self.context.as_ref().unwrap().lock() {
            Ok(guard) => guard,
            Err(_) => return cpu_fallback(),
        };

        let ctx = &*context_lock;

        let mut flattened_features = Vec::with_capacity(num_candidates * num_features);
        for feats in feature_batch {
            for i in 0..num_features {
                flattened_features.push(feats.get(i).cloned().unwrap_or(0.0));
            }
        }

        #[repr(C)]
        #[derive(Copy, Clone, bytemuck::Pod, bytemuck::Zeroable)]
        struct Params {
            num_candidates: u32,
            num_features: u32,
            is_nonlinear: u32,
            padding: u32,
        }

        let params = Params {
            num_candidates: num_candidates as u32,
            num_features: num_features as u32,
            is_nonlinear: if is_nonlinear { 1 } else { 0 },
            padding: 0,
        };

        ctx.queue.write_buffer(&ctx.params_buffer, 0, bytemuck::bytes_of(&params));
        ctx.queue.write_buffer(&ctx.weights_buffer, 0, bytemuck::cast_slice(weights));
        ctx.queue.write_buffer(&ctx.features_buffer, 0, bytemuck::cast_slice(&flattened_features));

        let scores_byte_size = (num_candidates * std::mem::size_of::<f32>()) as u64;

        let mut encoder = ctx.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Compute Command Encoder"),
        });

        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Evaluator Pass"),
                timestamp_writes: None,
            });
            compute_pass.set_pipeline(&ctx.pipeline);
            compute_pass.set_bind_group(0, &ctx.bind_group, &[]);
            let workgroup_count = (num_candidates as u32 + 63) / 64;
            compute_pass.dispatch_workgroups(workgroup_count, 1, 1);
        }

        encoder.copy_buffer_to_buffer(&ctx.scores_buffer, 0, &ctx.readback_buffer, 0, scores_byte_size);
        ctx.queue.submit(Some(encoder.finish()));

        let buffer_slice = ctx.readback_buffer.slice(0..scores_byte_size);
        let (sender, receiver) = std::sync::mpsc::channel();
        buffer_slice.map_async(wgpu::MapMode::Read, move |result| {
            let _ = sender.send(result);
        });

        ctx.device.poll(wgpu::Maintain::Wait);

        if let Ok(Ok(())) = receiver.recv() {
            let data = buffer_slice.get_mapped_range();
            let result_scores: &[f32] = bytemuck::cast_slice(&data);
            let vec_result = result_scores.to_vec();
            drop(data);
            ctx.readback_buffer.unmap();
            vec_result
        } else {
            cpu_fallback()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gpu_evaluator_initialization() {
        let evaluator = get_gpu_evaluator();
        println!("GPU Info: {}", evaluator.get_info_string());

        let weights = vec![1.0, 2.0, -1.0];
        let features = vec![
            vec![1.0, 0.5, 2.0], // 1.0*1.0 + 2.0*0.5 - 1.0*2.0 = 0.0
            vec![2.0, 1.0, 0.0], // 1.0*2.0 + 2.0*1.0 - 1.0*0.0 = 4.0
        ];

        let scores = evaluator.evaluate_batch(&weights, &features, false);
        assert_eq!(scores.len(), 2);
        assert!((scores[0] - 0.0).abs() < 1e-4);
        assert!((scores[1] - 4.0).abs() < 1e-4);
    }
}
