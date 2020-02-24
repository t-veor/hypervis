use zerocopy::{AsBytes, FromBytes};

#[repr(C)]
#[derive(Copy, Clone, Debug, FromBytes, AsBytes)]
pub struct Vertex4 {
    pub position: [f32; 4],
    pub color: [f32; 4],
}

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
                    offset: mem::size_of::<[f32; 4]>() as wgpu::BufferAddress,
                    shader_location: 1,
                    format: wgpu::VertexFormat::Float4,
                },
            ],
        }
    }
}

fn cube(
    size: f32,
    fixed_axis: usize,
    fixed_value: f32,
    color: [f32; 4],
    vertices: &mut Vec<Vertex4>,
    indices: &mut Vec<u32>,
) {
    let vertex_size = vertices.len() as u32;

    for mut i in 0..8 {
        let mut position = [0f32; 4];
        for j in 0..4 {
            if j == fixed_axis {
                position[j] = fixed_value * size;
            } else {
                position[j] = ((i & 1) as f32 * 2.0 - 1.0) * size;
                i /= 2;
            }
        }
        vertices.push(Vertex4 { position, color });
    }

    #[cfg_attr(rustfmt, rustfmt_skip)]
    let new_indices = vec![
        1, 2, 4, 7,
        0, 1, 2, 4,
        2, 4, 6, 7,
        1, 2, 3, 7,
        1, 4, 5, 7,
    ];

    indices.extend(new_indices.iter().map(|x| x + vertex_size));
}

fn tesseract_verts(size: f32) -> (Vec<Vertex4>, Vec<u32>) {
    let x = size / 2.0;

    let colors = &[
        [1.0, 0.0, 0.0, 1.0], // ana-side cube
        [0.0, 1.0, 1.0, 1.0], // kana-side cube
        [0.0, 1.0, 0.0, 1.0], // top-side cube
        [1.0, 0.0, 1.0, 1.0], // bottom-side cube
        [0.0, 0.0, 1.0, 1.0], // back-side cube
        [1.0, 1.0, 0.0, 1.0], // front-side cube
        [1.0, 0.5, 0.0, 1.0], // right-side cube
        [0.0, 0.5, 1.0, 1.0], // left-side cube
    ];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    // ana-side cube
    cube(x, 3, -1.0, colors[0], &mut vertices, &mut indices);
    // kata-side cube
    cube(x, 3, 1.0, colors[1], &mut vertices, &mut indices);
    // top-side cube
    cube(x, 2, -1.0, colors[2], &mut vertices, &mut indices);
    // bottom-side cube
    cube(x, 2, 1.0, colors[3], &mut vertices, &mut indices);
    // front-side cube
    cube(x, 1, -1.0, colors[4], &mut vertices, &mut indices);
    // back-side cube
    cube(x, 1, 1.0, colors[5], &mut vertices, &mut indices);
    // right-side cube
    cube(x, 0, -1.0, colors[6], &mut vertices, &mut indices);
    // left-side cube
    cube(x, 0, 1.0, colors[7], &mut vertices, &mut indices);

    (vertices, indices)
}

pub struct Mesh4 {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_buffer_size: wgpu::BufferAddress,
    pub index_buffer_size: wgpu::BufferAddress,
    pub simplex_count: u32,
}

pub fn tesseract(device: &wgpu::Device, size: f32) -> Mesh4 {
    let (vertices, indices) = tesseract_verts(size);

    let vertex_buffer_size = (vertices.len() * std::mem::size_of::<Vertex4>())
        as wgpu::BufferAddress;
    let index_buffer_size =
        (indices.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;

    let simplex_count = (indices.len() / 4) as u32;

    let vertex_buffer = device
        .create_buffer_mapped(
            vertices.len(),
            wgpu::BufferUsage::COPY_SRC
                | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::STORAGE_READ,
        )
        .fill_from_slice(&vertices);
    let index_buffer = device
        .create_buffer_mapped(
            indices.len(),
            wgpu::BufferUsage::COPY_SRC
                | wgpu::BufferUsage::COPY_DST
                | wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::STORAGE_READ,
        )
        .fill_from_slice(&indices);

    Mesh4 {
        vertex_buffer,
        index_buffer,
        vertex_buffer_size,
        index_buffer_size,
        simplex_count,
    }
}
