
use model::Model;
use wgpu::util::DeviceExt;
use winit::{
    event::*,
    event_loop::{ControlFlow, EventLoop},
    window::WindowBuilder,
};

use cgmath::{prelude::*, num_traits::Num};
use model::Vertex;

mod camera;
mod instance;
mod model; 
mod resources;
mod texture;
use crate::{block::{Block}};


pub async fn run(block: Block) {

    env_logger::init();
    let event_loop = EventLoop::new();
    let mut block_m = block.clone();

    
    let window = WindowBuilder::new()
        .with_decorations(true)
        .with_title("3D Cellular Automata")
        .with_inner_size(winit::dpi::LogicalSize::new(1980 as i32 , 1080))
        .build(&event_loop)
        .unwrap();
    
    let mut state = State::new(&window, block).await;
        // run()
    event_loop.run(move |event, _, control_flow| {

        match event {
            Event::RedrawRequested(window_id) if window_id == window.id() => {

                //if update of grid to occur
                if state.frame_num as f32 % state.frames_to_update as f32 == 0.0 {
                    block_m.update_grid();
                    let instance_data = block_m.instances.iter().map(|i| i.to_raw()).collect::<Vec<_>>();
                    state.instance_len = instance_data.len() as i32;
                    state.instance_buffer = state.device.create_buffer_init(
                        &wgpu::util::BufferInitDescriptor {
                            label: Some("Instance Buffer"),
                            contents: bytemuck::cast_slice(&instance_data),
                            usage: wgpu::BufferUsages::VERTEX,
                        }
                    );
                }

                state.update();
                match state.render(&block_m) {
                    Ok(_) => {}
                    // econfigure the surface if lost
                    Err(wgpu::SurfaceError::Lost) => state.resize(state.size),
                    // The system is out of memory, we should probably quit
                    Err(wgpu::SurfaceError::OutOfMemory) => *control_flow = ControlFlow::Exit,
                    // All other errors (Outdated, Timeout) should be resolved by the next frame
                    Err(e) => eprintln!("{:?}", e),
                }

            }
            Event::MainEventsCleared => {
                window.request_redraw();
            }
            Event::WindowEvent {
                ref event,
                window_id,
            } if window_id == window.id() => if !state.input(event) { 
                match event {
                    WindowEvent::KeyboardInput { 
                        input:
                        KeyboardInput {
                            state: ElementState::Pressed,
                            virtual_keycode: Some(VirtualKeyCode::Space),
                            ..
                        },
                    .. } => {
                    },
                    WindowEvent::CloseRequested | WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                state: ElementState::Pressed,
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    } => *control_flow = ControlFlow::Exit
                    ,
                    WindowEvent::Resized(physical_size) => {
                        state.resize(*physical_size);
                    }
                    WindowEvent::ScaleFactorChanged { new_inner_size, .. } => {
                        state.resize(**new_inner_size);
                    }
                    _ => {}
                }
            }
            _ => {}
        }
    });

}

// lib.rs
use winit::window::Window;

 
struct State {
    surface: wgpu::Surface,
    device: wgpu::Device,
    queue: wgpu::Queue,
    config: wgpu::SurfaceConfiguration,
    size: winit::dpi::PhysicalSize<u32>,
    render_pipeline: wgpu::RenderPipeline,
    camera: camera::Camera,
    camera_uniform: camera::CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: camera::CameraController,
    instance_len: i32,
    instance_buffer: wgpu::Buffer,
    obj_model: Model,
    depth_texture: texture::Texture,
    frame_num: i64,
    frames_to_update: i64,
}

impl State {
    fn update_instances(&mut self, instances: Vec<instance::Instance>) {
        let instance_data = instances.iter().map(|i| i.to_raw()).collect::<Vec<_>>();
        self.instance_buffer = self.device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );
    }
    // Creating some of the wgpu types requires async code
    async fn new(window: &Window, block: Block) -> Self {
        
        let size = window.inner_size();
        // The instance is a handle to our GPU
        // Backends::all => Vulkan + Metal + DX12 + Browser WebGPU
        let instance = wgpu::Instance::new(wgpu::Backends::all());
        let surface = unsafe { instance.create_surface(window) };
        let adapter = instance.request_adapter(
            &wgpu::RequestAdapterOptions {
                power_preference: wgpu::PowerPreference::default(),
                compatible_surface: Some(&surface),
                force_fallback_adapter: false,
            },
        ).await.unwrap();

        let (device, queue) = adapter.request_device(
            &wgpu::DeviceDescriptor {
                features: wgpu::Features::empty(),
                // WebGL doesn't support all of wgpu's features, so if
                // we're building for the web we'll have to disable some.
                limits: if cfg!(target_arch = "wasm32") {
                    wgpu::Limits::downlevel_webgl2_defaults()
                } else {
                    wgpu::Limits::default()
                },
                label: None,
            },
            None, // Trace path
        ).await.unwrap();

        let config = wgpu::SurfaceConfiguration {
            usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
            format: surface.get_preferred_format(&adapter).unwrap(),
            width: size.width,
            height: size.height,
            present_mode: wgpu::PresentMode::Fifo,
        };
        surface.configure(&device, &config);

        let shader = device.create_shader_module(&wgpu::ShaderModuleDescriptor {
            label: Some("Shader"),
            source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
        });

        let camera_bind_group_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
            entries: &[
                wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    visibility: wgpu::ShaderStages::VERTEX,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    count: None,
                }
            ],
            label: Some("camera_bind_group_layout"),
        });
        
        let render_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("Render Pipeline Layout"),
            bind_group_layouts: &[
                &camera_bind_group_layout,
                //texture layout goes here also
            ],
            push_constant_ranges: &[],
        });



        let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("Render Pipeline"),
            layout: Some(&render_pipeline_layout),
            vertex: wgpu::VertexState {
                module: &shader,
                entry_point: "vs_main", // what shader to call
                buffers: &[
                    model::ModelVertex::desc(),
                    instance::InstanceRaw::desc(),
                ], 
            },
            fragment: Some(wgpu::FragmentState { // optional, in charge of coloring.
                module: &shader,
                entry_point: "fs_main",
                targets: &[wgpu::ColorTargetState { // blend is saying we want to replace all old pixels with new rendered ones
                                                    //write mask is saying we want to use all colors.
                    format: config.format,
                    blend: Some(wgpu::BlendState::REPLACE),
                    write_mask: wgpu::ColorWrites::ALL,
                }],
            }),
            primitive: wgpu::PrimitiveState {
                topology: wgpu::PrimitiveTopology::TriangleList, // topology triangle list means that every 3 verticies will == one triangle this is standard.
                strip_index_format: None,
                front_face: wgpu::FrontFace::Ccw, // ccw means that the triangle is facing forward?
                                                  // if verticies are counter clockwise. (CCW)
                cull_mode: None, //anything not facing forward are "culled" more twhen covering buffer
                // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
                //wgpu::PolygonMode::Fill
                polygon_mode: wgpu::PolygonMode::Fill,
                // Requires Features::DEPTH_CLIP_CONTROL
                unclipped_depth: false,
                // Requires Features::CONSERVATIVE_RASTERIZATION
                conservative: false,
            },
                depth_stencil: Some(wgpu::DepthStencilState {
                    format: texture::Texture::DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less, // 1.
                    stencil: wgpu::StencilState::default(), // 2.
                    bias: wgpu::DepthBiasState::default(),
                }),// will be coming back to this.
                multisample: wgpu::MultisampleState {
                count: 1, // defines how many sammples we will be using. apparently complex
                mask: !0, // specifies which samples to be active. this case we are using all.
                alpha_to_coverage_enabled: false, // anti aliasing option?? set to none
            },
            multiview: None, // how many array layers the render attachments can have??? todo::.
        });

        let camera = camera::Camera {
            // position the camera one unit up and 2 units back
            // +z is out of the screen
            eye: (0.0, 10.0, 100.0).into(),
            // have it look at the origin
            target: (0.0, 0.0, 0.0).into(),
            // which way is "up"
            up: cgmath::Vector3::unit_y(),
            aspect: config.width as f32 / config.height as f32,
            fovy: 45.0,
            znear: 0.1,
            zfar: 1000.0,
        };

        // in new() after creating `camera`

        let mut camera_uniform = camera::CameraUniform::new();
        camera_uniform.update_view_proj(&camera);

        let camera_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Camera Buffer"),
                contents: bytemuck::cast_slice(&[camera_uniform]),
                usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
            }
        );

        let camera_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
            layout: &camera_bind_group_layout,
            entries: &[
                wgpu::BindGroupEntry {
                    binding: 0,
                    resource: camera_buffer.as_entire_binding(),
                }
            ],
            label: Some("camera_bind_group"),
        });
        let camera_controller = camera::CameraController::new(0.4);
        
        let instance_data: Vec<_> = block.instances.iter().map(|i| i.to_raw()).collect::<Vec<_>>();
        let instance_len = instance_data.len() as i32;
        let instance_buffer = device.create_buffer_init(
            &wgpu::util::BufferInitDescriptor {
                label: Some("Instance Buffer"),
                contents: bytemuck::cast_slice(&instance_data),
                usage: wgpu::BufferUsages::VERTEX,
            }
        );


        let obj_model = resources::load_model(
"cube.obj",
            &device,
            &queue,
        ).await.unwrap();

        let depth_texture = texture::Texture::create_depth_texture(&device, &config, "depth_texture");
        
        Self{
            surface,
            device,
            queue,
            config,
            size,
            render_pipeline,
            camera,
            camera_buffer,
            camera_bind_group,
            camera_uniform,
            camera_controller,
            instance_buffer,
            obj_model,
            depth_texture,
            frame_num : 0,
            frames_to_update : 10,
            instance_len
        }


    }

    fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
        if new_size.width > 0 && new_size.height > 0 {
            self.size = new_size;
            self.config.width = new_size.width;
            self.config.height = new_size.height;
            self.surface.configure(&self.device, &self.config);
            self.camera.aspect = new_size.width as f32 / new_size.height as f32;
        }
        self.depth_texture = texture::Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
    }

    fn input(&mut self, event: &WindowEvent) -> bool {
        self.camera_controller.process_events(event)
    }

    fn update(&mut self) {
       
        self.camera_controller.update_camera(&mut self.camera);
        self.camera_uniform.update_view_proj(&self.camera);
        self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
    }

    fn render(&mut self, block: &Block) -> Result<(), wgpu::SurfaceError> {
        let output = self.surface.get_current_texture()?;
        let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());
        let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
            label: Some("Render Encoder"),
        });
        {
            let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("Render Pass"),
                color_attachments: &[wgpu::RenderPassColorAttachment {
                    view: &view,
                    resolve_target: None,
                    ops: wgpu::Operations {
                        load: wgpu::LoadOp::Clear(wgpu::Color {
                            r: 0.2,
                            g: 0.2,
                            b: 0.2,
                            a: 1.0,
                        }),
                        store: true,
                    },
                }],
                depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
                    view: &self.depth_texture.view,
                    depth_ops: Some(wgpu::Operations {
                        load: wgpu::LoadOp::Clear(1.0),
                        store: true,
                    }),
                    stencil_ops: None,
                }),
            });
            // lib.rs
            render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_bind_group(0, &self.camera_bind_group, &[]);

            use model::DrawModel;
            render_pass.draw_mesh_instanced(&self.obj_model.meshes[0], 0..self.instance_len as u32);
}       
        self.frame_num += 1;
        // submit will accept anything that implements IntoIter
        self.queue.submit(std::iter::once(encoder.finish()));
        output.present();
    
        Ok(())
    }
}


