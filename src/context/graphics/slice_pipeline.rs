use super::{GraphicsContext, SlicePlane, Transform4, Vertex4};

use anyhow::{anyhow, Context, Result};

pub const MAX_VERTEX_SIZE: wgpu::BufferAddress = 65536;
pub const WORK_GROUP_SIZE: u32 = 256;

pub struct SlicePipeline {
    pipeline: wgpu::ComputePipeline,
    uniform_bind_group_layout: wgpu::BindGroupLayout,
    src_bind_group_layout: wgpu::BindGroupLayout,
    dst_bind_group_layout: wgpu::BindGroupLayout,
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct DrawIndirectCommand {
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

pub struct MeshBinding {
    uniform_bind_group: wgpu::BindGroup,
    src_bind_group: wgpu::BindGroup,
    dst_bind_group: wgpu::BindGroup,
    simplex_count: u32,
    slice_plane_buffer: wgpu::Buffer,
    transform_buffer: wgpu::Buffer,
    pub(super) indirect_command_buffer: wgpu::Buffer,
    pub(super) dst_vertex_buffer: wgpu::Buffer,
}

fn ceil_div(x: u32, y: u32) -> u32 {
    x / y + if x % y != 0 { 1 } else { 0 }
}

impl SlicePipeline {
    pub fn new(ctx: &GraphicsContext) -> Result<Self> {
        let shader_src = include_str!("shaders/slice.comp");
        let shader_spirv = glsl_to_spirv::compile(
            shader_src,
            glsl_to_spirv::ShaderType::Compute,
        )
        .map_err(|s| anyhow!(s))
        .context("Failed to compile 'shaders/slice.comp' into SPIR-V")?;
        let shader_data = wgpu::read_spirv(shader_spirv)
            .context("Failed to load 'shaders/slice.comp' into WGPU")?;
        let shader_module = ctx.device.create_shader_module(&shader_data);

        let uniform_bind_group_layout = ctx.device.create_bind_group_layout(
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

        let src_bind_group_layout = ctx.device.create_bind_group_layout(
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

        let dst_bind_group_layout = ctx.device.create_bind_group_layout(
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

        let pipeline_layout = ctx.device.create_pipeline_layout(
            &wgpu::PipelineLayoutDescriptor {
                bind_group_layouts: &[
                    &uniform_bind_group_layout,
                    &src_bind_group_layout,
                    &dst_bind_group_layout,
                ],
            },
        );

        let pipeline = ctx.device.create_compute_pipeline(
            &wgpu::ComputePipelineDescriptor {
                layout: &pipeline_layout,
                compute_stage: wgpu::ProgrammableStageDescriptor {
                    module: &shader_module,
                    entry_point: "main",
                },
            },
        );

        Ok(Self {
            pipeline,
            uniform_bind_group_layout,
            src_bind_group_layout,
            dst_bind_group_layout,
        })
    }

    pub fn create_mesh_binding(
        &self,
        ctx: &GraphicsContext,
        vertices: &Vec<Vertex4>,
        indices: &Vec<u32>,
    ) -> MeshBinding {
        let simplex_count = (indices.len() / 4) as u32;
        let vertex_buffer_size = (vertices.len()
            * std::mem::size_of::<Vertex4>())
            as wgpu::BufferAddress;
        let index_buffer_size =
            (indices.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;

        // overestimate of how many triangles can be generated
        let dst_vertex_buffer_size =
            (simplex_count * 12 * std::mem::size_of::<Vertex4>() as u32)
                as wgpu::BufferAddress;

        let slice_plane_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[SlicePlane::default()]);

        let transform_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[Transform4::default()]);

        let indirect_command_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::INDIRECT
                    | wgpu::BufferUsage::STORAGE
                    | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[DrawIndirectCommand::default()]);

        let dst_vertex_buffer =
            ctx.device.create_buffer(&wgpu::BufferDescriptor {
                size: dst_vertex_buffer_size,
                usage: wgpu::BufferUsage::STORAGE | wgpu::BufferUsage::VERTEX,
            });

        let simplex_count_buffer = ctx
            .device
            .create_buffer_mapped(
                1,
                wgpu::BufferUsage::UNIFORM | wgpu::BufferUsage::COPY_DST,
            )
            .fill_from_slice(&[simplex_count]);

        let vertex_buffer = ctx
            .device
            .create_buffer_mapped(
                vertices.len(),
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::STORAGE_READ,
            )
            .fill_from_slice(vertices);

        let index_buffer = ctx
            .device
            .create_buffer_mapped(
                indices.len(),
                wgpu::BufferUsage::COPY_DST | wgpu::BufferUsage::STORAGE_READ,
            )
            .fill_from_slice(indices);

        let src_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.src_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &simplex_count_buffer,
                            range: 0..std::mem::size_of::<u32>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &vertex_buffer,
                            range: 0..vertex_buffer_size,
                        },
                    },
                    wgpu::Binding {
                        binding: 2,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &index_buffer,
                            range: 0..index_buffer_size,
                        },
                    },
                ],
            });

        let uniform_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.uniform_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &slice_plane_buffer,
                            range: 0..std::mem::size_of::<SlicePlane>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &transform_buffer,
                            range: 0..std::mem::size_of::<Transform4>()
                                as wgpu::BufferAddress,
                        },
                    },
                ],
            });

        let dst_bind_group =
            ctx.device.create_bind_group(&wgpu::BindGroupDescriptor {
                layout: &self.dst_bind_group_layout,
                bindings: &[
                    wgpu::Binding {
                        binding: 0,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &indirect_command_buffer,
                            range: 0..std::mem::size_of::<u32>()
                                as wgpu::BufferAddress,
                        },
                    },
                    wgpu::Binding {
                        binding: 1,
                        resource: wgpu::BindingResource::Buffer {
                            buffer: &dst_vertex_buffer,
                            range: 0..dst_vertex_buffer_size,
                        },
                    },
                ],
            });

        MeshBinding {
            uniform_bind_group,
            src_bind_group,
            dst_bind_group,
            simplex_count,
            slice_plane_buffer,
            transform_buffer,
            indirect_command_buffer,
            dst_vertex_buffer,
        }
    }

    pub fn render_mesh(
        &self,
        ctx: &GraphicsContext,
        encoder: &mut wgpu::CommandEncoder,
        slice: &SlicePlane,
        transform: &Transform4,
        mesh: &MeshBinding,
    ) {
        // update slice
        let slice_staging_buffer = ctx
            .device
            .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&[*slice]);
        encoder.copy_buffer_to_buffer(
            &slice_staging_buffer,
            0,
            &mesh.slice_plane_buffer,
            0,
            std::mem::size_of::<SlicePlane>() as wgpu::BufferAddress,
        );

        // update transform
        let transform_staging_buffer = ctx
            .device
            .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&[*transform]);
        encoder.copy_buffer_to_buffer(
            &transform_staging_buffer,
            0,
            &mesh.transform_buffer,
            0,
            std::mem::size_of::<Transform4>() as wgpu::BufferAddress,
        );

        // reset indirect command buffer
        let command_staging_buffer = ctx
            .device
            .create_buffer_mapped(1, wgpu::BufferUsage::COPY_SRC)
            .fill_from_slice(&[DrawIndirectCommand::default()]);
        encoder.copy_buffer_to_buffer(
            &command_staging_buffer,
            0,
            &mesh.indirect_command_buffer,
            0,
            std::mem::size_of::<DrawIndirectCommand>() as wgpu::BufferAddress,
        );

        // Compute into the destination bind group
        let mut compute_pass = encoder.begin_compute_pass();
        compute_pass.set_pipeline(&self.pipeline);
        compute_pass.set_bind_group(0, &mesh.uniform_bind_group, &[]);
        compute_pass.set_bind_group(1, &mesh.src_bind_group, &[]);
        compute_pass.set_bind_group(2, &mesh.dst_bind_group, &[]);
        compute_pass.dispatch(
            ceil_div(mesh.simplex_count, WORK_GROUP_SIZE),
            1,
            1,
        );
    }
}
