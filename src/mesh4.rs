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

pub fn tesseract(size: f32) -> Mesh4 {
    let x = size / 2.0;

    let colors = &[
        [1.0, 0.0, 0.0, 1.0], // ana-side cube
        [0.0, 1.0, 1.0, 1.0], // kata-side cube
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

    Mesh4 { vertices, indices }
}

pub fn cell_120() -> Mesh4 {
    let (vertices, indices) = crate::geometry::cell_120_simplices();
    Mesh4 { vertices, indices }
}

pub fn cell_600() -> Mesh4 {
    let (vertices, indices) = crate::geometry::cell_600_simplices();
    Mesh4 { vertices, indices }
}
