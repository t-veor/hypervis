use cgmath::Vector4;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Vertex4 {
    pub position: Vector4<f32>,
    pub color: Vector4<f32>,
}

unsafe impl bytemuck::Pod for Vertex4 {}
unsafe impl bytemuck::Zeroable for Vertex4 {}

impl Vertex4 {
    pub fn desc<'a>() -> wgpu::VertexBufferDescriptor<'a> {
        use std::mem;
        wgpu::VertexBufferDescriptor {
            stride: mem::size_of::<Vertex4>() as wgpu::BufferAddress,
            step_mode: wgpu::InputStepMode::Vertex,
            attributes: &[
                wgpu::VertexAttributeDescriptor {
                    offset: 0,
                    shader_location: 0,
                    format: wgpu::VertexFormat::Float4,
                },
                wgpu::VertexAttributeDescriptor {
                    offset: mem::size_of::<Vector4<f32>>()
                        as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}
