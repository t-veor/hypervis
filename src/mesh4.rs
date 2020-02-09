use itertools::Itertools;
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

fn cube<I>(
    size: f32,
    x: &I,
    y: &I,
    z: &I,
    w: &I,
    color: [f32; 4],
    vertices: &mut Vec<Vertex4>,
    indices: &mut Vec<u32>,
) where
    I: Iterator<Item = i32> + Clone,
{
    let vertex_size = vertices.len() as u32;

    let it = x
        .clone()
        .cartesian_product(y.clone())
        .cartesian_product(z.clone())
        .cartesian_product(w.clone());
    for (((x, y), z), w) in it {
        vertices.push(Vertex4 {
            position: [
                (x as f32) * size,
                (y as f32) * size,
                (z as f32) * size,
                (w as f32) * size,
            ],
            color,
        });
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
    let x = size;

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

    let v = &(0..2);
    let a = &(0..1);
    let b = &(1..2);

    // ana-side cube
    cube(x, v, v, v, a, colors[0], &mut vertices, &mut indices);
    // kata-side cube
    cube(x, v, v, v, b, colors[1], &mut vertices, &mut indices);
    // top-side cube
    cube(x, v, v, a, v, colors[2], &mut vertices, &mut indices);
    // bottom-side cube
    cube(x, v, v, b, v, colors[3], &mut vertices, &mut indices);
    // front-side cube
    cube(x, v, a, v, v, colors[4], &mut vertices, &mut indices);
    // back-side cube
    cube(x, v, b, v, v, colors[5], &mut vertices, &mut indices);
    // right-side cube
    cube(x, a, v, v, v, colors[6], &mut vertices, &mut indices);
    // left-side cube
    cube(x, b, v, v, v, colors[7], &mut vertices, &mut indices);

    (vertices, indices)
}

pub struct Mesh4 {
    pub vertex_buffer: wgpu::Buffer,
    pub index_buffer: wgpu::Buffer,
    pub vertex_buffer_size: wgpu::BufferAddress,
    pub index_buffer_size: wgpu::BufferAddress,
}

pub fn tesseract(device: &wgpu::Device, size: f32) -> Mesh4 {
    let (vertices, indices) = tesseract_verts(size);

    let vertex_buffer_size = (vertices.len() * std::mem::size_of::<Vertex4>())
        as wgpu::BufferAddress;
    let index_buffer_size =
        (indices.len() * std::mem::size_of::<u32>()) as wgpu::BufferAddress;

    let vertex_buffer = device
        .create_buffer_mapped(
            vertices.len(),
            wgpu::BufferUsage::COPY_SRC
                | wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::STORAGE_READ,
        )
        .fill_from_slice(&vertices);
    let index_buffer = device
        .create_buffer_mapped(
            indices.len(),
            wgpu::BufferUsage::COPY_SRC
                | wgpu::BufferUsage::STORAGE
                | wgpu::BufferUsage::STORAGE_READ,
        )
        .fill_from_slice(&indices);

    Mesh4 {
        vertex_buffer,
        index_buffer,
        vertex_buffer_size,
        index_buffer_size,
    }
}
