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
    //cube(x, 3, 1.0, colors[1], &mut vertices, &mut indices);
    // top-side cube
    //cube(x, 2, -1.0, colors[2], &mut vertices, &mut indices);
    // bottom-side cube
    //cube(x, 2, 1.0, colors[3], &mut vertices, &mut indices);
    // front-side cube
    //cube(x, 1, -1.0, colors[4], &mut vertices, &mut indices);
    // back-side cube
    //cube(x, 1, 1.0, colors[5], &mut vertices, &mut indices);
    // right-side cube
    //cube(x, 0, -1.0, colors[6], &mut vertices, &mut indices);
    // left-side cube
    //cube(x, 0, 1.0, colors[7], &mut vertices, &mut indices);

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

pub type Triangle = [Vertex4; 3];
use super::CutPlane;
use nalgebra as na;

pub const EPSILON: f32 = 0.000001;

fn approx_zero(x: f32) -> bool {
    x.abs() < EPSILON
}

fn approx_equal(a: f32, b: f32) -> bool {
    (a - b).abs() < EPSILON
}

fn cut_line_seg(
    start: &na::Vector4<f32>,
    end: &na::Vector4<f32>,
    cut_plane: &CutPlane,
) -> Option<(na::Vector4<f32>, f32)> {
    let distance = cut_plane.normal.dot(&cut_plane.base_point);
    let b = end - start;
    let z = b.dot(&cut_plane.normal);

    if !approx_zero(z) {
        let t = (distance - start.dot(&cut_plane.normal)) / z;
        if -EPSILON < t && 1.0 - t > -EPSILON {
            let intersection = start + t * b;
            return Some((intersection, t));
        }
    }
    None
}

fn compute_cuts(
    vertices: &Vec<na::Vector4<f32>>,
    indices: &Vec<u32>,
    cut_plane: &CutPlane,
) {
    let detect_intersection = |a: &na::Vector4<f32>, b: &na::Vector4<f32>| {
        if let Some((v, t)) = cut_line_seg(a, b, cut_plane) {
            println!("{:?}, {}", cut_plane.proj_matrix * v, t);
        }
    };

    for i in (0..indices.len()).step_by(4) {
        println!("{}!", i);

        let a = vertices[indices[i + 0] as usize];
        let b = vertices[indices[i + 1] as usize];
        let c = vertices[indices[i + 2] as usize];
        let d = vertices[indices[i + 3] as usize];

        detect_intersection(&a, &b);
        detect_intersection(&c, &a);
        detect_intersection(&d, &a);
        detect_intersection(&b, &c);
        detect_intersection(&b, &d);
        detect_intersection(&c, &d);
    }
}

#[cfg(test)]
mod test {
    use super::super::*;
    use super::*;
    #[test]
    fn test() {
        let (vertices, indices) = tesseract_verts(1.0);
        let cut_plane = CutPlane {
            normal: na::Vector4::new(0.5, 0.5, 0.5, 0.5),
            base_point: na::Vector4::zeros(),
            proj_matrix: na::Matrix4::from_rows(&[
                na::RowVector4::new(0.5, 0.5, -0.5, -0.5),
                na::RowVector4::new(0.5, -0.5, 0.5, -0.5),
                na::RowVector4::new(0.5, -0.5, -0.5, 0.5),
                na::RowVector4::new(0.0, 0.0, 0.0, 0.0),
            ]),
        };
        let mut rotor = alg::Rotor4::identity();
        let angular_vel = alg::Bivec4::new(1.0, -1.0, 0.0, 0.0, -1.0, 1.0);

        for _ in 0..4 {
            let dt = 1f32 / 60f32;
            rotor.update(&(dt * angular_vel.clone()));
        }

        let rotation_matrix = rotor.to_matrix();
        let vertices = vertices
            .iter()
            .map(|v| {
                rotation_matrix * na::Vector4::from_column_slice(&v.position)
            })
            .collect();

        compute_cuts(&vertices, &indices, &cut_plane);
    }
}
