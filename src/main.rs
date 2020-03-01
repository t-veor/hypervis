mod alg;
mod context;
mod geometry;
mod mesh4;

use anyhow::Result;
use cgmath::{prelude::Zero, Matrix4, Point3, Vector3, Vector4};
use winit::event::WindowEvent;

use context::graphics::{
    MeshBinding, SlicePipeline, SlicePlane, Transform4, Vertex4,
};
use context::{Application, Ctx};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
struct ViewProjection {
    view_proj: Matrix4<f32>,
}

impl ViewProjection {
    pub fn new(ctx: &Ctx) -> Self {
        #[rustfmt::skip]
        let opengl_to_wgpu_matrix = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, -1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,
        );

        Self {
            view_proj: opengl_to_wgpu_matrix
                * cgmath::perspective(
                    cgmath::Deg(90.0),
                    ctx.graphics_ctx.sc_desc.width as f32
                        / ctx.graphics_ctx.sc_desc.height as f32,
                    0.1,
                    1000.0,
                )
                * Matrix4::look_at(
                    Point3::new(1.0, 1.0, -2.0),
                    Point3::new(0.0, 0.0, 0.0),
                    Vector3::unit_y(),
                ),
        }
    }
}
const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

struct TestApp {
    render_pipeline: wgpu::RenderPipeline,
    slice_pipeline: SlicePipeline,
    mesh_binding: MeshBinding,
    slice_plane: SlicePlane,
    depth_texture: wgpu::Texture,
    depth_texture_view: wgpu::TextureView,
    view_proj: ViewProjection,
    view_proj_buffer: wgpu::Buffer,
    vertex_bind_group: wgpu::BindGroup,
    angular_vel: alg::Bivec4,
    rotor: alg::Rotor4,
    frames: usize,
}

impl Application for TestApp {
    fn init(ctx: &mut Ctx) -> Self {
        let vs_src = include_str!("shader.vert");
        let fs_src = include_str!("shader.frag");

        let vs_spirv =
            glsl_to_spirv::compile(vs_src, glsl_to_spirv::ShaderType::Vertex)
                .unwrap();
        let fs_spirv =
            glsl_to_spirv::compile(fs_src, glsl_to_spirv::ShaderType::Fragment)
                .unwrap();

        let vs_data = wgpu::read_spirv(vs_spirv).unwrap();
        let fs_data = wgpu::read_spirv(fs_spirv).unwrap();

        let vs_module = ctx.graphics_ctx.device.create_shader_module(&vs_data);
        let fs_module = ctx.graphics_ctx.device.create_shader_module(&fs_data);

        let diagonal = SlicePlane {
            normal: Vector4::new(0.5, 0.5, 0.5, 0.5),
            base_point: Vector4::zero(),
            #[rustfmt::skip]
            proj_matrix: Matrix4::new(
                0.5, 0.5, 0.5, 0.0,
                0.5, -0.5, -0.5, 0.0,
                -0.5, 0.5, -0.5, 0.0,
                -0.5, -0.5, 0.5, 0.0,
            ),
        };

        let orthogonal = SlicePlane {
            normal: Vector4::new(0.0, 0.0, 0.0, 1.0),
            base_point: Vector4::new(0.0, 0.0, 0.0, 0.5),
            #[rustfmt::skip]
            proj_matrix: Matrix4::new(
                1.0, 0.0, 0.0, 0.0,
                0.0, 1.0, 0.0, 0.0,
                0.0, 0.0, 1.0, 0.0,
                0.0, 0.0, 0.0, 0.0,
            ),
        };

        let slice_plane = orthogonal;

        let rotor = alg::Rotor4::identity();
        let angular_vel = alg::Bivec4::new(0.0, 0.0, 0.1, 0.0, 0.1, 0.1);

        let slice_pipeline = SlicePipeline::new(&ctx.graphics_ctx).unwrap();

        let mesh = mesh4::cell_120();
        let mesh_binding = slice_pipeline.create_mesh_binding(
            &ctx.graphics_ctx,
            &mesh.vertices,
            &mesh.indices,
        );

        let view_proj = ViewProjection::new(ctx);

        let view_proj_buffer = ctx
            .graphics_ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[view_proj]);

        let depth_texture =
            ctx.graphics_ctx
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    ..ctx.graphics_ctx.sc_desc.to_texture_desc()
                });
        let depth_texture_view = depth_texture.create_default_view();

        let vertex_bind_group_layout = ctx
            .graphics_ctx
            .device
            .create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            });

        let vertex_bind_group = ctx.graphics_ctx.device.create_bind_group(
            &wgpu::BindGroupDescriptor {
                layout: &vertex_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &view_proj_buffer,
                        range: 0..std::mem::size_of::<ViewProjection>()
                            as wgpu::BufferAddress,
                    },
                }],
            },
        );

        let render_pipeline_layout = ctx
            .graphics_ctx
            .device
            .create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[&vertex_bind_group_layout],
            });

        let render_pipeline = ctx.graphics_ctx.device.create_render_pipeline(
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
                    format: ctx.graphics_ctx.sc_desc.format,
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
                vertex_buffers: &[Vertex4::desc()],
                sample_count: 1,
                sample_mask: !0,
                alpha_to_coverage_enabled: false,
            },
        );

        TestApp {
            render_pipeline,
            slice_pipeline,
            slice_plane,
            mesh_binding,
            depth_texture,
            depth_texture_view,
            view_proj,
            view_proj_buffer,
            vertex_bind_group,
            rotor,
            angular_vel,
            frames: 0,
        }
    }

    fn resize(&mut self, ctx: &mut Ctx) {
        let mut encoder = ctx.graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );
        // update the projection
        {
            self.view_proj = ViewProjection::new(ctx);
            let staging_buffer = ctx
                .graphics_ctx
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
        ctx.graphics_ctx.queue.submit(&[encoder.finish()]);

        self.depth_texture =
            ctx.graphics_ctx
                .device
                .create_texture(&wgpu::TextureDescriptor {
                    format: DEPTH_FORMAT,
                    usage: wgpu::TextureUsage::OUTPUT_ATTACHMENT,
                    ..ctx.graphics_ctx.sc_desc.to_texture_desc()
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
        let mut encoder = ctx.graphics_ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );

        // Update the slice
        let scale = (self.frames % 600) as f32 / 600.0 * 2.0 - 1.0;
        self.slice_plane.base_point = Vector4::new(0.0, 0.0, 0.0, scale);

        // Update the rotation
        // println!("{}", self.frames);
        let dt = 1f32 / 60f32;
        self.rotor.update(&(dt * self.angular_vel.clone()));
        // println!("{:?}", self.rotor);
        let rotation_matrix = self.rotor.to_matrix();
        // println!("{}", rotation_matrix);
        let transform = Transform4 {
            displacement: Vector4::zero(),
            transform: rotation_matrix,
        };

        self.slice_pipeline.render_mesh(
            &ctx.graphics_ctx,
            &mut encoder,
            &self.slice_plane,
            &transform,
            &self.mesh_binding,
        );

        // for some reason I need to do the compute and render passes in two
        // goes to have it work on vulkan without visual glitches
        ctx.graphics_ctx.queue.submit(&[encoder.finish()]);

        let frame = ctx.graphics_ctx.swap_chain.get_next_texture();
        let mut encoder = ctx.graphics_ctx.device.create_command_encoder(
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
            render_pass.set_vertex_buffers(
                0,
                &[(self.slice_pipeline.dst_vertex_buffer(), 0)],
            );
            render_pass.set_bind_group(0, &self.vertex_bind_group, &[]);
            render_pass.draw_indirect(
                self.slice_pipeline.indirect_command_buffer(),
                0,
            );
        }

        ctx.graphics_ctx.queue.submit(&[encoder.finish()]);

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
