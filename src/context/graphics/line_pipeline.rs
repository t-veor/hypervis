use super::{
    GraphicsContext, Transform4, Vertex4, ViewProjection3, ViewProjection4,
    DEPTH_FORMAT,
};
use crate::mesh::Mesh;

use anyhow::{anyhow, Context, Result};
use cgmath::Vector4;

pub struct LinePipeline {
    pipeline: wgpu::RenderPipeline,
    view_proj3_buffer: wgpu::Buffer,
    view_proj4_buffer: wgpu::Buffer,
    transform_bind_group_layout: wgpu::BindGroupLayout,
    uniform_bind_group: wgpu::BindGroup,
}

pub struct LineBinding {
    vertex_buffer: wgpu::Buffer,
    index_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    transform_bind_group: wgpu::BindGroup,
    num_indices: u32,
}

impl LinePipeline {
    pub fn new(ctx: &GraphicsContext) -> Result<Self> {
        let vs_src = include_str!("shaders/line.vert");
        let fs_src = include_str!("shaders/shader.frag");

        let vs_spirv =
            glsl_to_spirv::compile(vs_src, glsl_to_spirv::ShaderType::Vertex)
                .map_err(|s| anyhow!(s))
                .context("Failed to compile 'shaders/shader.vert' to SPIR-V")?;
        let fs_spirv =
            glsl_to_spirv::compile(fs_src, glsl_to_spirv::ShaderType::Fragment)
                .map_err(|s| anyhow!(s))
                .context("Failed to compile 'shaders/shader.frag' to SPIR-V")?;

        let vs_data = wgpu::read_spirv(vs_spirv)?;
        let fs_data = wgpu::read_spirv(fs_spirv)?;

        let vs_module = ctx.device.create_shader_module(&vs_data);
        let fs_module = ctx.device.create_shader_module(&fs_data);

        let view_proj3_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[ViewProjection3::default()]);

        let view_proj4_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[ViewProjection4::default()]);

        let uniform_bind_group_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[
                    wgpu::BindGroupLayoutBinding {
                        binding: 0,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                    wgpu::BindGroupLayoutBinding {
                        binding: 1,
                        visibility: wgpu::ShaderStage::VERTEX,
                        ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                    },
                ],
            },
        );

        let uniform_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &uniform_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &view_proj3_buffer,
                            range: 0..std::mem::size_of::<ViewProjection3>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &view_proj4_buffer,
                            range: 0..std::mem::size_of::<ViewProjection4>()
                                as wgpu::BufferAddress,
                        },
                    },
                ],
            });

        let transform_bind_group_layout = ctx.device.create_bind_group_layout(
            &wgpu::BindGroupLayoutDescriptor {
                bindings: &[wgpu::BindGroupLayoutBinding {
                    binding: 0,
                    visibility: wgpu::ShaderStage::VERTEX,
                    ty: wgpu::BindingType::UniformBuffer { dynamic: false },
                }],
            },
        );

        let pipeline_layout = ctx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    &transform_bind_group_layout,
                ],
            },
        );

        let pipeline = ctx.device.create_render_pipeline(
            &wgpu::RenderPipelineDescriptor {
                layout: &pipeline_layout,
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
                primitive_topology: wgpu::PrimitiveTopology::LineList,
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

        Ok(Self {
            pipeline,
            view_proj3_buffer,
            view_proj4_buffer,
            uniform_bind_group,
            transform_bind_group_layout,
        })
    }

    pub fn create_binding(
        &self,
        ctx: &GraphicsContext,
        mesh: &Mesh,
        color: Vector4<f32>,
    ) -> LineBinding {
        let vertices: Vec<_> = mesh
            .vertices
            .iter()
            .map(|v| Vertex4 {
                position: *v,
                color,
            })
            .collect();

        let mut indices = Vec::new();
        indices.reserve(mesh.edges.len() * 2);
        for e in mesh.edges.iter() {
            indices.push(e.hd_vertex as u16);
            indices.push(e.tl_vertex as u16);
        }

        let vertex_buffer = ctx
            .device
            .create_buffer_mapped(vertices.len(), wgpu::BufferUsage::VERTEX)
            .fill_from_slice(&vertices);

        let index_buffer = ctx
            .device
            .create_buffer_mapped(indices.len(), wgpu::BufferUsage::INDEX)
            .fill_from_slice(&indices);

        let num_indices = indices.len() as u32;

        let transform_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::UNIFORM,
            )
            .fill_from_slice(&[Transform4::default()]);

        let transform_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.transform_bind_group_layout,
                bindings: &[wgpu::Binding {
                    binding: 0,
                    resource: wgpu::BindingResource::Buffer {
                        buffer: &transform_buffer,
                        range: 0..std::mem::size_of::<Transform4>()
                            as wgpu::BufferAddress,
                    },
                }],
            });

        LineBinding {
            vertex_buffer,
            index_buffer,
            num_indices,
            transform_buffer,
            transform_bind_group,
        }
    }

    pub fn update_view_proj(
        &self,
        ctx: &mut GraphicsContext,
        view_proj3: &ViewProjection3,
        view_proj4: &ViewProjection4,
    ) {
        let mut encoder = ctx.device.create_command_encoder(
            &wgpu::CommandEncoderDescriptor { todo: 0 },
        );
        // update the projection
        {
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[*view_proj3]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.view_proj3_buffer,
                0,
                std::mem::size_of::<ViewProjection3>() as wgpu::BufferAddress,
            );
        }
        {
            let staging_buffer = ctx
                .device
                .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
                .fill_from_slice(&[*view_proj4]);
            encoder.copy_buffer_to_buffer(
                &staging_buffer,
                0,
                &self.view_proj4_buffer,
                0,
                std::mem::size_of::<ViewProjection4>() as wgpu::BufferAddress,
            );
        }
        ctx.queue.submit(&[encoder.finish()]);
    }

    pub fn update_binding(
        &self,
        ctx: &GraphicsContext,
        encoder: &mut wgpu::CommandEncoder,
        transform: &Transform4,
        binding: &LineBinding,
    ) {
        let staging_buffer = ctx
            .device
            .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&[*transform]);
        encoder.copy_buffer_to_buffer(
            &staging_buffer,
            0,
            &binding.transform_buffer,
            0,
            std::mem::size_of::<Transform4>() as wgpu::BufferAddress,
        );
    }

    pub fn render(
        &self,
        render_pass: &mut wgpu::RenderPass,
        binding: &LineBinding,
    ) {
        render_pass.set_pipeline(&self.pipeline);
        render_pass.set_vertex_buffers(0, &[(&binding.vertex_buffer, 0)]);
        render_pass.set_index_buffer(&binding.index_buffer, 0);
        render_pass.set_bind_group(0, &self.uniform_bind_group, &[]);
        render_pass.set_bind_group(1, &binding.transform_bind_group, &[]);
        render_pass.draw_indexed(0..binding.num_indices, 0, 0..1);
    }
}
