use winit::window::Window;
use winit::event::*;

use wgpu::util::DeviceExt;

use cgmath::prelude::*;

use crate::{
	texture::*,
	camera::*,
	instance::*,
	model::*,
	resources::*,
};

const NUM_INSTANCES_PER_ROW: u32 = 10;
const INSTANCE_DISPLACEMENT: cgmath::Vector3<f32> = cgmath::Vector3::new(NUM_INSTANCES_PER_ROW as f32 * 0.5, 0.0, NUM_INSTANCES_PER_ROW as f32 * 0.5);
const SPACE_BETWEEN: f32 = 3.0;

// #[repr(C)]
// #[derive(Copy, Clone, Debug, bytemuck::Pod, bytemuck::Zeroable)]
// struct Vertex {
// 	position: [f32; 3],
// 	tex_coords: [f32; 2],
// }

// impl Vertex {
//     fn desc<'a>() -> wgpu::VertexBufferLayout<'a> {
//         wgpu::VertexBufferLayout {
//             array_stride: std::mem::size_of::<Vertex>() as wgpu::BufferAddress,
//             step_mode: wgpu::VertexStepMode::Vertex,
//             attributes: &[
//                 wgpu::VertexAttribute {
//                     offset: 0,
//                     shader_location: 0,
//                     format: wgpu::VertexFormat::Float32x3,
//                 },
//                 wgpu::VertexAttribute {
//                     offset: std::mem::size_of::<[f32; 3]>() as wgpu::BufferAddress,
//                     shader_location: 1,
//                     format: wgpu::VertexFormat::Float32x2,
//                 }
//             ]
//         }
//     }
// }

// const VERTICES: &[Vertex] = &[
//     Vertex { position: [-0.0868241, 0.49240386, 0.0], tex_coords: [0.4131759, 0.00759614], }, // A
//     Vertex { position: [-0.49513406, 0.06958647, 0.0], tex_coords: [0.0048659444, 0.43041354], }, // B
//     Vertex { position: [-0.21918549, -0.44939706, 0.0], tex_coords: [0.28081453, 0.949397], }, // C
//     Vertex { position: [0.35966998, -0.3473291, 0.0], tex_coords: [0.85967, 0.84732914], }, // D
//     Vertex { position: [0.44147372, 0.2347359, 0.0], tex_coords: [0.9414737, 0.2652641], }, // E
// ];


const INDICES: &[u16] = &[
    0, 1, 4,
    1, 2, 4,
    2, 3, 4,
];

pub struct State {
	surface: wgpu::Surface,
	device: wgpu::Device,
	queue: wgpu::Queue,
	config: wgpu::SurfaceConfiguration,
	pub size: winit::dpi::PhysicalSize<u32>,
	render_pipeline: wgpu::RenderPipeline,
	// vertex_buffer: wgpu::Buffer,
	// num_vertices: u32,
    // index_buffer: wgpu::Buffer, 
    // num_indices: u32,
    diffuse_bind_group: wgpu::BindGroup,
    diffuse_texture: Texture,
    camera: Camera,
    camera_uniform: CameraUniform,
    camera_buffer: wgpu::Buffer,
    camera_bind_group: wgpu::BindGroup,
    camera_controller: CameraController,
	instances: Vec<Instance>,
	instance_buffer: wgpu::Buffer,
	depth_texture: Texture,
	obj_model: Model,
}

impl  State {
	
	pub async fn new(window: &Window) -> Self {
		let size = window.inner_size();

		let instance = wgpu::Instance::new(wgpu::Backends::all());
		let surface = unsafe { instance.create_surface(window) };
		let adapter = instance.request_adapter(
			&wgpu::RequestAdapterOptions {
				power_preference: wgpu::PowerPreference::default(),
				compatible_surface: Some(&surface),
				force_fallback_adapter: false,
			}
		).await.unwrap();

		let (device, queue) = adapter.request_device(
			
			&wgpu::DeviceDescriptor {
				features: wgpu::Features::empty(),
				limits: wgpu::Limits::default(),
				label: None,
			},

			None,

		).await.unwrap();

		let config = wgpu::SurfaceConfiguration {
			usage: wgpu::TextureUsages::RENDER_ATTACHMENT,
			format: surface.get_supported_formats(&adapter)[0],
			width: size.width,
			height: size.height,
			present_mode: wgpu::PresentMode::Fifo,
			alpha_mode: wgpu::CompositeAlphaMode::Auto,
		};

		surface.configure(&device, &config);
		
		let diffuse_bytes = include_bytes!("../ferris.png");
		let diffuse_texture = Texture::from_bytes(&device, &queue, diffuse_bytes, "../ferris.png").unwrap();

		let texture_bind_group_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                entries: &[
                    wgpu::BindGroupLayoutEntry {
                        binding: 0,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        ty: wgpu::BindingType::Texture {
                            multisampled: false,
                            view_dimension: wgpu::TextureViewDimension::D2,
                            sample_type: wgpu::TextureSampleType::Float { filterable: true },
                        },
                        count: None,
                    },
                    wgpu::BindGroupLayoutEntry {
                        binding: 1,
                        visibility: wgpu::ShaderStages::FRAGMENT,
                        // This should match the filterable field of the
                        // corresponding Texture entry above.
                        ty: wgpu::BindingType::Sampler(wgpu::SamplerBindingType::Filtering),
                        count: None,
                    },
                ],
                label: Some("texture_bind_group_layout"),
            });


		let diffuse_bind_group = device.create_bind_group(
		    &wgpu::BindGroupDescriptor {
		        layout: &texture_bind_group_layout,
		        entries: &[
		            wgpu::BindGroupEntry {
		                binding: 0,
		                resource: wgpu::BindingResource::TextureView(&diffuse_texture.view),
		            },
		            wgpu::BindGroupEntry {
		                binding: 1,
		                resource: wgpu::BindingResource::Sampler(&diffuse_texture.sampler),
		            }
		        ],
		        label: Some("diffuse_bind_group"),
		    }
		);

	    let camera = Camera {
	        // position the camera one unit up and 2 units back
	        // +z is out of the screen
	        eye: (0.0, 1.0, 2.0).into(),
	        // have it look at the origin
	        target: (0.0, 0.0, 0.0).into(),
	        // which way is "up"
	        up: cgmath::Vector3::unit_y(),
	        aspect: config.width as f32 / config.height as f32,
	        fovy: 45.0,
	        znear: 0.1,
	        zfar: 100.0,
	    };

	    let mut camera_uniform = CameraUniform::new();
		camera_uniform.update_view_proj(&camera);

		let camera_buffer = device.create_buffer_init(
		    &wgpu::util::BufferInitDescriptor {
		        label: Some("Camera Buffer"),
		        contents: bytemuck::cast_slice(&[camera_uniform]),
		        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
		    }
		);

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

		let camera_controller = CameraController::new(0.2);

		let instances = (0..NUM_INSTANCES_PER_ROW).flat_map(|z| {
		    (0..NUM_INSTANCES_PER_ROW).map(move |x| {
		        let x = SPACE_BETWEEN * (x as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);
		        let z = SPACE_BETWEEN * (z as f32 - NUM_INSTANCES_PER_ROW as f32 / 2.0);

		        let position = cgmath::Vector3 { x, y: 0.0, z };

		        let rotation = if position.is_zero() {
		            cgmath::Quaternion::from_axis_angle(cgmath::Vector3::unit_z(), cgmath::Deg(0.0))
		        } else {
		            cgmath::Quaternion::from_axis_angle(position.normalize(), cgmath::Deg(45.0))
		        };

		        Instance {
		            position, rotation,
		        }
		    })
		}).collect::<Vec<_>>();

        let instance_data = instances.iter().map(Instance::to_raw).collect::<Vec<_>>();
		let instance_buffer = device.create_buffer_init(
		    &wgpu::util::BufferInitDescriptor {
		        label: Some("Instance Buffer"),
		        contents: bytemuck::cast_slice(&instance_data),
		        usage: wgpu::BufferUsages::VERTEX,
		    }
		);

		let depth_texture = Texture::create_depth_texture(&device, &config, "depth_texture");

		let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
		    label: Some("Shader"),
		    source: wgpu::ShaderSource::Wgsl(include_str!("shader.wgsl").into()),
		});

		let render_pipeline_layout =
		    device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
		        label: Some("Render Pipeline Layout"),
		        bind_group_layouts: &[
		        	&texture_bind_group_layout,
		        	&camera_bind_group_layout,
		        ],
		        push_constant_ranges: &[],
		    });

		let render_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
		    label: Some("Render Pipeline"),
		    layout: Some(&render_pipeline_layout),
		    vertex: wgpu::VertexState {
		        module: &shader,
		        entry_point: "vs_main",
		        buffers: &[
		        	ModelVertex::desc(), InstanceRaw::desc()
		        ],
		    },
		    fragment: Some(wgpu::FragmentState {
		        module: &shader,
		        entry_point: "fs_main",
		        targets: &[Some(wgpu::ColorTargetState {
		            format: config.format,
		            blend: Some(wgpu::BlendState::REPLACE),
		            write_mask: wgpu::ColorWrites::ALL,
		        })],
		    }),
		    primitive: wgpu::PrimitiveState {
		        topology: wgpu::PrimitiveTopology::TriangleList,
		        strip_index_format: None,
		        front_face: wgpu::FrontFace::Ccw,
		        cull_mode: Some(wgpu::Face::Back),
		        // Setting this to anything other than Fill requires Features::NON_FILL_POLYGON_MODE
		        polygon_mode: wgpu::PolygonMode::Fill,
		        // Requires Features::DEPTH_CLIP_CONTROL
		        unclipped_depth: false,
		        // Requires Features::CONSERVATIVE_RASTERIZATION
		        conservative: false,
		    },
		    depth_stencil: Some(wgpu::DepthStencilState {
		        format: Texture::DEPTH_FORMAT,
		        depth_write_enabled: true,
		        depth_compare: wgpu::CompareFunction::Less, // 1.
		        stencil: wgpu::StencilState::default(), // 2.
		        bias: wgpu::DepthBiasState::default(),
		    }),
		    multisample: wgpu::MultisampleState {
		        count: 1,
		        mask: !0,
		        alpha_to_coverage_enabled: false,
		    },
		    multiview: None,
		});

		let obj_model = load_model(
		    "teapot.obj",
		    &device,
		    &queue,
		    &texture_bind_group_layout,
		).await.unwrap();

		Self {
			surface, device, queue,
			config, size, render_pipeline,
			diffuse_bind_group, diffuse_texture,
			camera, camera_uniform, camera_buffer, camera_bind_group, camera_controller,
			instances, instance_buffer,
			depth_texture, obj_model
		}

	}

	pub fn resize(&mut self, new_size: winit::dpi::PhysicalSize<u32>) {
		if new_size.width > 0 && new_size.height > 0 {
			self.size = new_size;
			self.config.width = new_size.width;
			self.config.height = new_size.height;
			self.surface.configure(&self.device, &self.config);
			self.depth_texture = Texture::create_depth_texture(&self.device, &self.config, "depth_texture");
		}
	}

	pub fn input(&mut self, event: &WindowEvent) -> bool {
		self.camera_controller.process_events(event)
	}

	pub fn update(&mut self) {
		self.camera_controller.update_camera(&mut self.camera);
    	self.camera_uniform.update_view_proj(&self.camera);
    	self.queue.write_buffer(&self.camera_buffer, 0, bytemuck::cast_slice(&[self.camera_uniform]));
	}

	pub fn render(&mut self) -> Result<(), wgpu::SurfaceError> {
		let output = self.surface.get_current_texture()?;

		let view = output.texture.create_view(&wgpu::TextureViewDescriptor::default());

		let mut encoder = self.device.create_command_encoder(&wgpu::CommandEncoderDescriptor {
			label: Some("Render Encoder"),
		});


		let mut render_pass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
			label: Some("Render Pass"),
			color_attachments: &[Some(wgpu::RenderPassColorAttachment {
				view: &view,
				resolve_target: None,
				ops: wgpu::Operations {
					load: wgpu::LoadOp::Clear(wgpu::Color {
						r: 0.1,
						g: 0.2,
						b: 0.3,
						a: 1.0,
					}),
					store: true,
				},
			})],
		    depth_stencil_attachment: Some(wgpu::RenderPassDepthStencilAttachment {
		        view: &self.depth_texture.view,
		        depth_ops: Some(wgpu::Operations {
		            load: wgpu::LoadOp::Clear(1.0),
		            store: true,
		        }),
		        stencil_ops: None,
		    }),
		});

		render_pass.set_pipeline(&self.render_pipeline);

		render_pass.set_bind_group(0, &self.diffuse_bind_group, &[]);
		render_pass.set_bind_group(1, &self.camera_bind_group, &[]);
			
		render_pass.set_vertex_buffer(1, self.instance_buffer.slice(..));
		
		render_pass.draw_model_instanced(&self.obj_model, 0..self.instances.len() as u32, &self.camera_bind_group);

		drop(render_pass);

		self.queue.submit(std::iter::once(encoder.finish()));
		output.present();

		Ok(())

	}
}