use crate::context::graphics::Vertex4;

pub struct Mesh4 {
    pub vertices: Vec<Vertex4>,
    pub indices: Vec<u32>,
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
        vertices.push(Vertex4 {
            position: position.into(),
            color: color.into(),
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

pub fn floor(size: f32) -> Mesh4 {
    let x = size / 2.0;
    let color = [1.0f32; 4];

    let mut vertices = Vec::new();
    let mut indices = Vec::new();

    cube(x, 1, 0.0, color, &mut vertices, &mut indices);

    Mesh4 { vertices, indices }
}
