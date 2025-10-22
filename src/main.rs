use std::sync::Arc;

use eframe::egui_wgpu::{
    self,
    RenderState,
};
use egui::Vec2;
use tracing::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let fmt_subscriber = tracing_subscriber::fmt::Subscriber::builder()
        .with_max_level(tracing::Level::DEBUG)
        .finish();
    tracing::subscriber::set_global_default(fmt_subscriber)?;

    info!("Hello world!");
    let native_options = eframe::NativeOptions::default();
    eframe::run_native(
        "floating",
        native_options,
        Box::new(|cc| Ok(Box::new(EguiApp::new(cc)))),
    )?;

    Ok(())
}

struct EguiApp {}

impl EguiApp {
    #[allow(clippy::too_many_lines)]
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        let wgpu = cc.wgpu_render_state.clone().expect("Not using wgpu");

        let compute_layout =
            wgpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label:   Some("gray_compute_layout"),
                    entries: &[wgpu::BindGroupLayoutEntry {
                        binding:    0,
                        visibility: wgpu::ShaderStages::COMPUTE,
                        ty:         wgpu::BindingType::StorageTexture {
                            access:         wgpu::StorageTextureAccess::WriteOnly,
                            format:         wgpu::TextureFormat::Rgba8Unorm,
                            view_dimension: wgpu::TextureViewDimension::D2,
                        },
                        count:      None,
                    }],
                });
        let compute_pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label:                None,
                    bind_group_layouts:   &[&compute_layout],
                    push_constant_ranges: &[],
                });

        let compute_shader = wgpu
            .device
            .create_shader_module(wgpu::include_wgsl!("compute.wgsl"));
        let compute_pipeline =
            wgpu.device
                .create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
                    label:               Some("ray_pipeline"),
                    layout:              Some(&compute_pipeline_layout),
                    module:              &compute_shader,
                    entry_point:         None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    cache:               None,
                });

        let render_shader = wgpu
            .device
            .create_shader_module(wgpu::include_wgsl!("render.wgsl"));
        let render_layout =
            wgpu.device
                .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                    label:   Some("gray_render_layout"),
                    entries: &[
                        wgpu::BindGroupLayoutEntry {
                            binding:    0,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty:         wgpu::BindingType::Texture {
                                sample_type:    wgpu::TextureSampleType::Float { filterable: true },
                                view_dimension: wgpu::TextureViewDimension::D2,
                                multisampled:   false,
                            },
                            count:      None,
                        },
                        wgpu::BindGroupLayoutEntry {
                            binding:    1,
                            visibility: wgpu::ShaderStages::FRAGMENT,
                            ty:         wgpu::BindingType::Sampler(
                                wgpu::SamplerBindingType::Filtering,
                            ),
                            count:      None,
                        },
                    ],
                });
        let render_pipeline_layout =
            wgpu.device
                .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                    label:                None,
                    bind_group_layouts:   &[&render_layout],
                    push_constant_ranges: &[],
                });

        let render_pipeline = wgpu
            .device
            .create_render_pipeline(&wgpu::RenderPipelineDescriptor {
                label:         Some("gray_render_pipeline"),
                layout:        Some(&render_pipeline_layout),
                vertex:        wgpu::VertexState {
                    module:              &render_shader,
                    entry_point:         None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    buffers:             &[],
                },
                primitive:     wgpu::PrimitiveState {
                    topology:           wgpu::PrimitiveTopology::TriangleStrip,
                    strip_index_format: None,
                    front_face:         wgpu::FrontFace::Ccw,
                    cull_mode:          None,
                    unclipped_depth:    false,
                    polygon_mode:       wgpu::PolygonMode::Fill,
                    conservative:       false,
                },
                depth_stencil: None,
                multisample:   wgpu::MultisampleState {
                    count:                     1,
                    mask:                      !0,
                    alpha_to_coverage_enabled: false,
                },
                fragment:      Some(wgpu::FragmentState {
                    module:              &render_shader,
                    entry_point:         None,
                    compilation_options: wgpu::PipelineCompilationOptions::default(),
                    targets:             &[Some(wgpu.target_format.into())],
                }),
                multiview:     None,
                cache:         None,
            });

        wgpu.renderer
            .write()
            .callback_resources
            .insert(RayTracerContext {
                compute_pipeline,
                compute_layout,

                render_pipeline,
                render_layout,
                render_bindgroup: None,

                texture: None,
            });

        Self {}
    }

    fn render(ui: &mut egui::Ui) {
        let (rect, _response) = ui.allocate_at_least(ui.available_size(), egui::Sense::empty());

        let callback = egui_wgpu::Callback::new_paint_callback(rect, RayTracerRenderer {});
        ui.painter().add(callback);
    }
}

impl eframe::App for EguiApp {
    fn update(
        &mut self,
        ctx: &egui::Context,
        _frame: &mut eframe::Frame,
    ) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Hello from egui!");

            egui::Frame::canvas(ui.style()).show(ui, Self::render)
        });
    }
}

struct RayTracerContext {
    render_pipeline:  wgpu::RenderPipeline,
    render_layout:    wgpu::BindGroupLayout,
    render_bindgroup: Option<wgpu::BindGroup>,

    compute_pipeline: wgpu::ComputePipeline,
    compute_layout:   wgpu::BindGroupLayout,

    texture: Option<wgpu::Texture>,
}

struct RayTracerRenderer {}

impl egui_wgpu::CallbackTrait for RayTracerRenderer {
    fn prepare(
        &self,
        device: &wgpu::Device,
        _queue: &wgpu::Queue,
        screen_descriptor: &egui_wgpu::ScreenDescriptor,
        encoder: &mut wgpu::CommandEncoder,
        callback_resources: &mut egui_wgpu::CallbackResources,
    ) -> Vec<wgpu::CommandBuffer> {
        let ctx: &mut RayTracerContext =
            callback_resources.get_mut().expect("Scene not intialized");
        let [width, height] = screen_descriptor.size_in_pixels;

        let mut new_texture = || {
            ctx.render_bindgroup = None;
            device.create_texture(&wgpu::TextureDescriptor {
                label:           Some("gray_output"),
                size:            wgpu::Extent3d {
                    width,
                    height,
                    depth_or_array_layers: 1,
                },
                mip_level_count: 1,
                sample_count:    1,
                dimension:       wgpu::TextureDimension::D2,
                format:          wgpu::TextureFormat::Rgba8Unorm,
                usage:           wgpu::TextureUsages::TEXTURE_BINDING
                    | wgpu::TextureUsages::STORAGE_BINDING,
                view_formats:    &[wgpu::TextureFormat::Rgba8Unorm],
            })
        };

        let texture = ctx.texture.get_or_insert_with(&mut new_texture);
        if texture.width() != width || texture.height() != height {
            *texture = new_texture();
        }

        let texture_view = texture.create_view(&wgpu::TextureViewDescriptor::default());

        let compute_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            label:   Some("gray_compute"),
            layout:  &ctx.compute_layout,
            entries: &[wgpu::BindGroupEntry {
                binding:  0,
                resource: wgpu::BindingResource::TextureView(&texture_view),
            }],
        });

        if ctx.render_bindgroup.is_none() {
            let texture_sampler = device.create_sampler(&wgpu::SamplerDescriptor {
                address_mode_u: wgpu::AddressMode::ClampToEdge,
                address_mode_v: wgpu::AddressMode::ClampToEdge,
                address_mode_w: wgpu::AddressMode::ClampToEdge,
                mag_filter: wgpu::FilterMode::Linear,
                min_filter: wgpu::FilterMode::Nearest,
                mipmap_filter: wgpu::FilterMode::Nearest,
                ..Default::default()
            });
            let render_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                label:   Some("gray_render"),
                layout:  &ctx.render_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding:  0,
                        resource: wgpu::BindingResource::TextureView(&texture_view),
                    },
                    wgpu::BindGroupEntry {
                        binding:  1,
                        resource: wgpu::BindingResource::Sampler(&texture_sampler),
                    },
                ],
            });
            ctx.render_bindgroup = Some(render_bind_group);
        }

        {
            let mut pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label:            Some("ray_tracer_pass"),
                timestamp_writes: None,
            });
            pass.set_pipeline(&ctx.compute_pipeline);
            pass.set_bind_group(0, &compute_bind_group, &[]);
            pass.dispatch_workgroups(width, height, 1);
        }

        Vec::new()
    }

    fn paint(
        &self,
        _info: egui::PaintCallbackInfo,
        render_pass: &mut wgpu::RenderPass<'static>,
        callback_resources: &egui_wgpu::CallbackResources,
    ) {
        let ctx: &RayTracerContext = callback_resources.get().expect("Scene not intialized");

        render_pass.set_pipeline(&ctx.render_pipeline);
        render_pass.set_bind_group(
            0,
            ctx.render_bindgroup
                .as_ref()
                .expect("Bindgroup not initalized"),
            &[],
        );
        render_pass.draw(0..4, 0..1);
    }
}
