use pollster::FutureExt;
use wgpu::util::DeviceExt;

const WAIT_TIME: std::time::Duration = std::time::Duration::from_millis(500);

use winit::{
    event::*,
    event_loop::{ActiveEventLoop, EventLoop},
    keyboard::{KeyCode, PhysicalKey},
    window::{Window, WindowId},
};

struct StateApplication<'a> {
    state: Option<State<'a>>,
}

impl<'a> StateApplication<'a> {
    pub fn new() -> Self {
        Self { state: None }
    }
}

pub async fn run() {
    env_logger::init();
    let event_loop = EventLoop::new().unwrap();
    let mut window_state = StateApplication::new();
    let _ = event_loop.run_app(&mut window_state);
}

impl<'a> winit::application::ApplicationHandler for StateApplication<'a> {
    fn resumed(&mut self, event_loop: &ActiveEventLoop) {
        let window = event_loop
            .create_window(Window::default_attributes().with_title("Test"))
            .unwrap();
        self.state = Some(State::new(window));
    }

    fn window_event(
        &mut self,
        event_loop: &ActiveEventLoop,
        window_id: WindowId,
        event: WindowEvent,
    ) {
        let window = self.state.as_ref().unwrap().window();

        if window.id() == window_id {
            match event {
                WindowEvent::CloseRequested => {
                    event_loop.exit();
                }
                WindowEvent::Resized(physical_size) => {
                    self.state.as_mut().unwrap().resize(physical_size);
                }
                WindowEvent::RedrawRequested => {
                    self.state.as_mut().unwrap().render().unwrap();
                }
                _ => {}
            }
        }
    }

    fn about_to_wait(&mut self, event_loop: &ActiveEventLoop) {
        let window = self.state.as_ref().unwrap().window();
        std::thread::sleep(WAIT_TIME);
        event_loop.set_control_flow(winit::event_loop::ControlFlow::Poll);
        window.request_redraw();
    }
}

struct State<'a> {
    window: std::sync::Arc<Window>,
    surface: wgpu::Surface<'a>,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    vertex_buffer: wgpu::Buffer,
    cell_bind_group: [wgpu::BindGroup; 2],
    compute_bind_group: [wgpu::BindGroup; 2],
    step: u8,
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct Vertex {
    position: [f32; 2],
}

impl Vertex {
    fn desc() -> wgpu::VertexBufferLayout<'static> {
        wgpu::VertexBufferLayout {
            array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
            step_mode: wgpu::VertexStepMode::Vertex,
            attributes: &[wgpu::VertexAttribute {
                offset: 0,
                shader_location: 0,
                format: wgpu::VertexFormat::Float32x2,
            }],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
struct GridUniform {
    data: [f32; 2],
}

#[rustfmt::skip]
const VERTICES: &[Vertex] = &[
    Vertex { position: [-0.8, -0.8]},
    Vertex { position: [0.8, -0.8]},
    Vertex { position: [0.8, 0.8]},
    Vertex { position: [-0.8, -0.8]},
    Vertex { position: [0.8, 0.8]},
    Vertex { position: [-0.8, 0.8]}
];

const GRID_SIZE: f32 = 32.0;

#[rustfmt::skip]
const GRID: &[GridUniform] = &[
    GridUniform { data: [GRID_SIZE, GRID_SIZE] },
];

impl<'a> State<'a> {
    fn new(window_arc: Window) -> Self {
        let window = std::sync::Arc::new(window_arc);
        let size = window.inner_size();

        let instance = wgpu::Instance::new(wgpu::InstanceDescriptor {
            backends: wgpu::Backends::VULKAN,
            ..Default::default()
        });

        let surface = instance.create_surface(window.clone()).unwrap();
        let adapter = instance
            .request_adapter(&wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            })
            .block_on()
            .unwrap();

        let (device, queue) = adapter
            .request_device(
                &wgpu::DeviceDescriptor {
                    required_features: wgpu::Features::empty(),
                    required_limits: wgpu::Limits::default(),
                    label: None,
                    memory_hints: Default::default(),
                },
                None,
            )
            .block_on()
            .unwrap();

        let surface_caps = surface.get_capabilities(&adapter);
        let surface_format = surface_caps
            .formats
            .iter()
            .find(|f| f.is_srgb())
            .copied()
            .unwrap_or(surface_caps.formats[0]);

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface_format,
            width: size.width,
            height: size.height,
            present_mode: surface_caps.present_modes[0],
            alpha_mode: surface_caps.alpha_modes[0],
            view_formats: vec![],
            desired_maximum_frame_latency: 2,
        };

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Vertex Buffer"),
            contents: bytemuck::cast_slice(VERTICES),
            usage: wgpu::BufferUsages::VERTEX,
        });

        let grid_uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("Grid uniform buffer"),
            contents: bytemuck::cast_slice(GRID),
            usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
        });

        let mut cell_state: Vec<u32> = vec![0; (GRID_SIZE * GRID_SIZE) as usize];

        for index in 0..cell_state.len() {
            cell_state[index] = if rand::random::<f32>() > 0.6 { 1 } else { 0 };
        }

        let cell_state_storage_buffer = [
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cell state A buffer"),
                contents: bytemuck::cast_slice(&cell_state),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
            device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
                label: Some("Cell state B buffer"),
                contents: bytemuck::cast_slice(&cell_state),
                usage: wgpu::BufferUsages::STORAGE | wgpu::BufferUsages::COPY_DST,
            }),
        ];

        let cell_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::VERTEX | wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Uniform,
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::VERTEX,
                        ty: wgpu::BindingType::Buffer {
                            ty: wgpu::BufferBindingType::Storage { read_only: true },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("Cell binding group layout"),
            });

        let compute_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
                            ty: wgpu::BufferBindingType::Storage { read_only: false },
                            has_dynamic_offset: false,
                            min_binding_size: None,
                        },
                        count: None,
                    },
                ],
                label: Some("Compute bind group layout"),
            });

        let cell_bind_group = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &cell_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: grid_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: cell_state_storage_buffer[0].as_entire_binding(),
                    },
                ],
                label: Some("Cell Bind group A"),
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &cell_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: grid_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: cell_state_storage_buffer[1].as_entire_binding(),
                    },
                ],
                label: Some("Cell Bind group B"),
            }),
        ];

        let compute_bind_group = [
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: grid_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: cell_state_storage_buffer[0].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cell_state_storage_buffer[1].as_entire_binding(),
                    },
                ],
                label: Some("Compute bind group A"),
            }),
            device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_bind_group_layout,
                entries: &[
                    wgpu::BindGroupEntry {
                        binding: 0,
                        resource: grid_uniform_buffer.as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 1,
                        resource: cell_state_storage_buffer[1].as_entire_binding(),
                    },
                    wgpu::BindGroupEntry {
                        binding: 2,
                        resource: cell_state_storage_buffer[0].as_entire_binding(),
                    },
                ],
                label: Some("Compute bind group B"),
            }),
        ];

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/shader.wgsl").into()),
        });

        let compute = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("Compute shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../shaders/compute.wgsl").into()),
        });

        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Render pipeline Layout"),
                bind_group_layouts: &[&cell_bind_group_layout],
                push_constant_ranges: &[],
            });

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("Compute pipeline layout"),
                bind_group_layouts: &[&compute_bind_group_layout],
                push_constant_ranges: &[],
            });

        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: Some("vertexMain"),
                buffers: &[Vertex::desc()],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            },
            fragment: Some(wgpu::FragmentState {
                module: &shader,
                entry_point: Some("fragmentMain"),
                targets: &[Some(wgpu::ColorTargetState {
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                })],
                compilation_options: wgpu::PipelineCompilationOptions::default(),
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList,
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw,
                cull_mode: Some(wgpu::Face::Back),
                // Setting this to anything other than Fill requires Features::POLYGON_MODE_LINE
                // or Features::POLYGON_MODE_POINT
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
            depth_stencil: None,
            multisample: wgpu::MultisampleState {
                count: 1,
                mask: !0,
                alpha_to_coverage_enabled: false,
            },
            multiview: None,
            cache: None,
        });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("Compute pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute,
            entry_point: Some("computeMain"),
            compilation_options: Default::default(),
            cache: None,
        });

        Self {
            window,
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            compute_pipeline,
            vertex_buffer,
            cell_bind_group,
            compute_bind_group,
            step: 0,
        }
    }

    pub fn window(&self) -> &Window {
        &self.window
    }
    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
        }
    }
    fn input(&mut self, _event: &WindowEvent) -> bool {
        false
    }
    fn update(&mut self) {
        //todo!()
    }
    fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
        self.step = self.step.wrapping_add(1);
        let output = self.surface.get_current_texture()?;
        let view = output
            .texture
            .create_view(&wgpu::TextureViewDescriptor::default());

        let mut encoder = self
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor {
                label: Some("Render Encoder"),
            });
        // COMPUTE PASS
        {
            let mut compute_pass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor {
                label: Some("Compute pass"),
                timestamp_writes: None,
            });

            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &self.compute_bind_group[(&self.step % 2) as usize],
                &[],
            );
            let workgroup_count = (GRID_SIZE / 8.0).ceil() as u32;
            compute_pass.dispatch_workgroups(workgroup_count, workgroup_count, 1);
        }
        // RENDER PASS
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.0,
                            g: 0.0,
                            b: 0.0,
                            a: 0.0,
                        }),
                        store: wgpu::StoreOp::Store,
                    },
                })],
                depth_stencil_attachment: None,
                occlusion_query_set: None,
                timestamp_writes: None,
            });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.cell_bind_group[(self.step % 2) as usize], &[]);
            render_pass.set_vertex_buffer(0, self.vertex_buffer.slice(..));
            render_pass.draw(
                0..VERTICES.len() as u32,
                0..((GRID_SIZE * GRID_SIZE) as u32),
            );
        }
        self.queue.submit(Some(encoder.finish()));
        output.present();

        Ok(())
    }
}
