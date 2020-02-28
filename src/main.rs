mod alg;
mod context;
mod geometry;
mod mesh4;

use anyhow::Result;
use nalgebra as na;
use winit::event::WindowEvent;
use zerocopy::{AsBytes, FromBytes};

use context::{Application, Ctx};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct CutPlane {
    normal: na::Vector4<f32>,
    base_point: na::Vector4<f32>,
    proj_matrix: na::Matrix4<f32>,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ViewProjection {
    view_proj: na::Matrix4<f32>,
}

impl ViewProjection {
    pub fn new(ctx: &Ctx) -> Self {
        #[cfg_attr(rustfmt, rustfmt_skip)]
        let opengl_to_wgpu_matrix = na::Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, -1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,
        );

        Self {
            view_proj: opengl_to_wgpu_matrix
                * na::Perspective3::new(
                    ctx.sc_desc.width as f32 / ctx.sc_desc.height as f32,
                    45.0,
                    0.1,
                    1000.0,
                )
                .as_matrix()
                * na::Isometry3::look_at_rh(
                    &na::Point3::new(1.0, 1.0, -2.0),
                    &na::Point3::new(0.0, 0.0, 0.0),
                    &na::Vector3::y(),
                )
                .to_homogeneous(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy, FromBytes, AsBytes)]
struct DrawIndirectCommand {
    vertex_count: u32,
    instance_count: u32,
    first_vertex: u32,
    first_instance: u32,
}

impl Default for DrawIndirectCommand {
    fn default() -> Self {
        Self {
            vertex_count: 0,
            instance_count: 1,
            first_vertex: 0,
            first_instance: 0,
        }
    }
}

const MAX_VERTEX_SIZE: wgpu::BufferAddress = 65536;
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

struct TestApp {
    render_pipeline: wgpu::RenderPipeline,
    compute_pipeline: wgpu::ComputePipeline,
    mesh: mesh4::Mesh4,
    cut_plane: CutPlane,
    cut_plane_buffer: wgpu::Buffer,
    simplex_count: wgpu::Buffer,
    draw_indirect_command: wgpu::Buffer,
    dst_vertices: wgpu::Buffer,
    compute_uniforms_bind_group: wgpu::BindGroup,
    compute_src_bind_group: wgpu::BindGroup,
    compute_dst_bind_group: wgpu::BindGroup,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    view_proj: ViewProjection,
    view_proj_buffer: wgpu::Buffer,
    vertex_bind_group: wgpu::BindGroup,
    angular_vel: alg::Bivec4,
    rotor: alg::Rotor4,
    rotation_matrix_buffer: wgpu::Buffer,
    frames: usize,
}

impl Application for TestApp {
    fn init(ctx: &mut Ctx) -> Self {
        let vs_src = include_str!("shader.vert");
        let fs_src = include_str!("shader.frag");
        let cs_src = include_str!("shader.comp");

        let vs_spirv =
            glsl_to_spirv::compile(vs_src, glsl_to_spirv::ShaderType::Vertex)
                .unwrap();
        let fs_spirv =
            glsl_to_spirv::compile(fs_src, glsl_to_spirv::ShaderType::Fragment)
                .unwrap();
        let cs_spirv =
            glsl_to_spirv::compile(cs_src, glsl_to_spirv::ShaderType::Compute)
                .unwrap();

        let vs_data = wgpu::read_spirv(vs_spirv).unwrap();
        let fs_data = wgpu::read_spirv(fs_spirv).unwrap();
        let cs_data = wgpu::read_spirv(cs_spirv).unwrap();

        let vs_module = ctx.device.create_shader_module(&vs_data);
        let fs_module = ctx.device.create_shader_module(&fs_data);
        let cs_module = ctx.device.create_shader_module(&cs_data);

        let compute_uniform_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                ],
            },
        );

        let compute_src_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: true,
                        },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 2,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: true,
                        },
                    },
                ],
            },
        );

        let compute_dst_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                        },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStage::COMPUTE,
                        ty: wgpu::BindingType::StorageBuffer {
                            dynamic: false,
                            readonly: false,
                        },
                    },
                ],
            },
        );

        let compute_pipeline_layout = ctx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &compute_uniform_layout,
                    &compute_src_layout,
                    &compute_dst_layout,
                ],
            },
        );

        let compute_pipeline = ctx.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                layout: &compute_pipeline_layout,
                compute_stage: wgpu::ProgrammableStageDescriptor {
                    module: &cs_module,
                    entry_point: "main",
                },
            },
        );

        let diagonal = CutPlane {
            normal: na::Vector4::new(0.5, 0.5, 0.5, 0.5),
            base_point: na::Vector4::zeros(),
            proj_matrix: na::Matrix4::from_rows(&[
                na::RowVector4::new(0.5, 0.5, -0.5, -0.5),
                na::RowVector4::new(0.5, -0.5, 0.5, -0.5),
                na::RowVector4::new(0.5, -0.5, -0.5, 0.5),
                na::RowVector4::new(0.0, 0.0, 0.0, 0.0),
            ]),
        };

        let orthogonal = CutPlane {
            normal: na::Vector4::new(0.0, 0.0, 0.0, 1.0),
            base_point: na::Vector4::new(0.0, 0.0, 0.0, 0.5),
            proj_matrix: na::Matrix4::from_rows(&[
                na::RowVector4::new(1.0, 0.0, 0.0, 0.0),
                na::RowVector4::new(0.0, 1.0, 0.0, 0.0),
                na::RowVector4::new(0.0, 0.0, 1.0, 0.0),
                na::RowVector4::new(0.0, 0.0, 0.0, 0.0),
            ]),
        };

        let cut_plane = orthogonal;

        let cut_plane_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[cut_plane]);

        let mut rotor = alg::Rotor4::identity();
        let angular_vel = alg::Bivec4::new(0.0, 0.0, 0.1, 0.0, 0.1, 0.1);

        /*
        for _ in 0..214 {
            let dt = 1f32 / 60f32;
            rotor.update(&(dt * angular_vel.clone()));
        }
        */

        let rotation_matrix = rotor.to_matrix();

        println!("{:?}\n{}", rotor, rotation_matrix);

        let rotation_matrix_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[rotation_matrix]);

        let mesh = mesh4::cell_120(&ctx.device);
        println!("{}", mesh.simplex_count);

        let simplex_count = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[mesh.simplex_count]);

        let draw_indirect_command = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::INDIRECT
                    | wgpu::BufferUsage::STORAGE
                    | wgpu::BufferUsage::COPY_DST
                    | wgpu::BufferUsage::MAP_READ,
            )
            .fill_from_slice(&[DrawIndirectCommand::default()]);

        let dst_vertices = ctx.device.create_buffer(&wgpu::BufferDescriptor {
            size: MAX_VERTEX_SIZE
                * std::mem::size_of::<mesh4::Vertex4>() as wgpu::BufferAddress,
            usage: wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::VERTEX
                | wgpu::BufferUsage::MAP_READ,
        });

        let compute_uniforms_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_uniform_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &cut_plane_buffer,
                            range: 0..std::mem::size_of_val(&cut_plane)
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &rotation_matrix_buffer,
                            range: 0..std::mem::size_of_val(&rotation_matrix)
                                as wgpu::BufferAddress,
                        },
                    },
                ],
            });

        let compute_src_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_src_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &simplex_count,
                            range: 0..std::mem::size_of::<u32>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &mesh.vertex_buffer,
                            range: 0..mesh.vertex_buffer_size,
                        },
                    },
                    wgpu::Binding {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &mesh.index_buffer,
                            range: 0..mesh.index_buffer_size,
                        },
                    },
                ],
            });

        let compute_dst_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &compute_dst_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &draw_indirect_command,
                            range: 0..std::mem::size_of::<u32>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &dst_vertices,
                            range: 0..MAX_VERTEX_SIZE
                                * std::mem::size_of::<mesh4::Vertex4>()
                                    as wgpu::BufferAddress,
                        },
                    },
                ],
            });

        let view_proj = ViewProjection::new(ctx);

        let view_proj_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[view_proj]);

        let depth_texture =
            ctx.device.create_texture(&wgpu::TextureDescriptor {
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                ..ctx.sc_desc.to_texture_desc()
            });
        let depth_texture_view = depth_texture.create_default_view();

        let vertex_bind_group_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            },
        );

        let vertex_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &vertex_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &view_proj_buffer,
                        range: 0..std::mem::size_of::<ViewProjection>()
                            as wgpu::BufferAddress,
                    },
                }],
            });

        let render_pipeline_layout = ctx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&vertex_bind_group_layout],
            },
        );

        let render_pipeline = ctx.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                layout: &render_pipeline_layout,
                vertex_stage: wgpu::ProgrammableStageDescriptor {
                    module: &vs_module,
                    entry_point: "main",
                },
                fragment_stage: Some(wgpu::ProgrammableStageDescriptor {
                    module: &fs_module,
                    entry_point: "main",
                }),
                rasterization_state: Some(wgpu::RasterizationStateDescriptor {
                    front_face: wgpu::FrontFace::Ccw,
                    cull_mode: wgpu::CullMode::None,
                    depth_bias: 0,
                    depth_bias_slope_scale: 0.0,
                    depth_bias_clamp: 0.0,
                }),
                color_states: &[wgpu::ColorStateDescriptor {
                    format: ctx.sc_desc.format,
                    color_blend: wgpu::BlendDescriptor::REPLACE,
                    alpha_blend: wgpu::BlendDescriptor::REPLACE,
                    write_mask: wgpu::ColorWrite::ALL,
                }],
                primitive_topology: wgpu::PrimitiveTopology::TriangleList,
                depth_stencil_state: Some(wgpu::DepthStencilStateDescriptor {
                    format: DEPTH_FORMAT,
                    depth_write_enabled: true,
                    depth_compare: wgpu::CompareFunction::Less,
                    stencil_front: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_back: wgpu::StencilStateFaceDescriptor::IGNORE,
                    stencil_read_mask: 0,
                    stencil_write_mask: 0,
                }),
                index_format: wgpu::IndexFormat::Uint16,
                vertex_buffers: &[mesh4::Vertex4::desc()],
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        );

        TestApp {
            render_pipeline,
            compute_pipeline,
            mesh,
            cut_plane,
            cut_plane_buffer,
            draw_indirect_command,
            dst_vertices,
            simplex_count,
            compute_uniforms_bind_group,
            compute_src_bind_group,
            compute_dst_bind_group,
            depth_texture,
            depth_texture_view,
            view_proj,
            view_proj_buffer,
            vertex_bind_group,
            rotor,
            angular_vel,
            rotation_matrix_buffer,
            frames: 0,
        }
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        let mut encoder = ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );
        // update the projection
        {
            self.view_proj = ViewProjection::new(ctx);
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[self.view_proj]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.view_proj_buffer,
                0,
                std::mem::size_of::<ViewProjection>() as wgpu::BufferAddress,
            );
        }
        ctx.queue.submit(&[encoder.finish()]);

        self.depth_texture =
            ctx.device.create_texture(&wgpu::TextureDescriptor {
                format: DEPTH_FORMAT,
                usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                ..ctx.sc_desc.to_texture_desc()
            });
        self.depth_texture_view = self.depth_texture.create_default_view();
    }

    fn on_event(&mut self, _ctx: &mut Ctx, event: WindowEvent) {
        match event {
            _ => (),
        }
    }

    fn update(&mut self, _ctx: &mut Ctx) {}

    fn render(&mut self, ctx: &mut Ctx) {
        let frame = ctx.swap_chain.get_next_texture();
        let mut encoder = ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        {
            let mut render_pass =
                encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                    color_attachments: &[
                        wgpu::RenderPassColorAttachmentDescriptor {
                            attachment: &frame.view,
                            resolve_target: None,
                            load_op: wgpu::LoadOp::Clear,
                            store_op: wgpu::StoreOp::Store,
                            clear_color: wgpu::Color {
                                r: 0.0,
                                g: 0.0,
                                b: 0.0,
                                a: 1.0,
                            },
                        },
                    ],
                    depth_stencil_attachment: Some(
                        wgpu::RenderPassDepthStencilAttachmentDescriptor {
                            attachment: &self.depth_texture_view,
                            depth_load_op: wgpu::LoadOp::Clear,
                            depth_store_op: wgpu::StoreOp::Store,
                            clear_depth: 1.0,
                            stencil_load_op: wgpu::LoadOp::Clear,
                            stencil_store_op: wgpu::StoreOp::Store,
                            clear_stencil: 0,
                        },
                    ),
                });

            render_pass.set_pipeline(&self.render_pipeline);
            render_pass.set_vertex_buffers(0, &[(&self.dst_vertices, 0)]);
            render_pass.set_bind_group(0, &self.vertex_bind_group, &[]);
            render_pass.draw_indirect(&self.draw_indirect_command, 0);
        }

        {
            let scale = (self.frames % 600) as f32 / 600.0 * 2.0 - 1.0;
            self.cut_plane.base_point = na::Vector4::new(0.0, 0.0, 0.0, scale);
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[self.cut_plane]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.cut_plane_buffer,
                0,
                std::mem::size_of::<CutPlane>() as wgpu::BufferAddress,
            );
        }

        // Update the rotation
        if true {
            // println!("{}", self.frames);
            let dt = 1f32 / 60f32;
            self.rotor.update(&(dt * self.angular_vel.clone()));
            // println!("{:?}", self.rotor);
            let rotation_matrix = self.rotor.to_matrix();
            // println!("{}", rotation_matrix);
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[rotation_matrix]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.rotation_matrix_buffer,
                0,
                std::mem::size_of_val(&rotation_matrix) as wgpu::BufferAddress,
            );
        }

        // reset the indirect command buffer
        {
            let indirect_command = DrawIndirectCommand::default();
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[indirect_command]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.draw_indirect_command,
                0,
                std::mem::size_of::<DrawIndirectCommand>()
                    as wgpu::BufferAddress,
            );
        }

        {
            let mut compute_pass = encoder.begin_compute_pass();
            compute_pass.set_pipeline(&self.compute_pipeline);
            compute_pass.set_bind_group(
                0,
                &self.compute_uniforms_bind_group,
                &[],
            );
            compute_pass.set_bind_group(1, &self.compute_src_bind_group, &[]);
            compute_pass.set_bind_group(2, &self.compute_dst_bind_group, &[]);
            compute_pass.dispatch(
                (self.mesh.simplex_count as f64 / 256.0).ceil() as u32,
                1,
                1,
            );
        }

        ctx.queue.submit(&[encoder.finish()]);

        self.frames += 1;

        /*
        self.draw_indirect_command.map_read_async(
            0,
            std::mem::size_of::<DrawIndirectCommand>() as wgpu::BufferAddress,
            |result: wgpu::BufferMapAsyncResult<&[DrawIndirectCommand]>| {
                if let Ok(e) = result {
                    println!("{:?}", e.data);
                }
            },
        );

        self.dst_vertices.map_read_async(
            0,
            288 * std::mem::size_of::<mesh4::Vertex4>() as wgpu::BufferAddress,
            |result: wgpu::BufferMapAsyncResult<&[mesh4::Vertex4]>| {
                if let Ok(e) = result {
                    println!("{:?}", e.data);
                }
            },
        );
        */
    }
}

fn main() -> Result<()> {
    context::run::<TestApp>("Hello world!", (800, 600))
}
