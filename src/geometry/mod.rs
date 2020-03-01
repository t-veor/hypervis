mod cell120;
mod cell600;

pub use cell120::*;
pub use cell600::*;

use crate::context::graphics::Vertex4;
use cgmath::{prelude::Zero, InnerSpace, Vector3, Vector4};

fn cell_120_hplanes() -> Vec<(Vector4<f32>, f32)> {
    cell_600_verts().iter().map(|v| (*v, -0.925615)).collect()
}

fn cell_600_hplanes() -> Vec<(Vector4<f32>, f32)> {
    cell_120_verts().iter().map(|v| (*v, -0.925615)).collect()
}

fn cell_120_cells() -> Vec<(Vector4<f32>, Vec<usize>)> {
    let vertices = cell_120_verts();
    let faces = cell_120_faces();
    cell_120_hplanes()
        .iter()
        .map(|(n, d)| {
            let mut faces_in_this_cell = Vec::new();

            for face_idx in (0..faces.len()).step_by(5) {
                let vertex_indices = &faces[face_idx..face_idx + 5];

                let mut inside = true;
                for vertex_idx in vertex_indices.iter() {
                    let vertex = vertices[*vertex_idx];
                    if vertex.dot(*n) - d > 0.00001 {
                        inside = false;
                        break;
                    }
                }

                if inside {
                    faces_in_this_cell.push(face_idx)
                }
            }

            (*n, faces_in_this_cell)
        })
        .collect()
}

fn cell_600_cells() -> Vec<(Vector4<f32>, Vec<usize>)> {
    let vertices = cell_600_verts();
    let faces = cell_600_faces();
    cell_600_hplanes()
        .iter()
        .map(|(n, d)| {
            let mut faces_in_this_cell = Vec::new();

            for face_idx in (0..faces.len()).step_by(3) {
                let vertex_indices = &faces[face_idx..face_idx + 3];

                let mut inside = true;
                for vertex_idx in vertex_indices.iter() {
                    let vertex = vertices[*vertex_idx];
                    if vertex.dot(*n) - d > 0.00001 {
                        inside = false;
                        break;
                    }
                }

                if inside {
                    faces_in_this_cell.push(face_idx)
                }
            }

            (*n, faces_in_this_cell)
        })
        .collect()
}

fn sort_points_on_plane(
    normal: &Vector4<f32>,
    points: &Vec<(usize, Vector4<f32>)>,
) -> Vec<(usize, Vector4<f32>)> {
    // find the axis x, y, z, or w which separates the points the most
    let vec4_to_arr = |v: &Vector4<f32>| [v.x, v.y, v.z, v.w];

    let normal_arr = vec4_to_arr(normal);
    let mut max_idx = 0;
    for i in 1..4 {
        if normal_arr[i].abs() > normal_arr[max_idx].abs() {
            max_idx = i;
        }
    }

    let projected_points: Vec<_> = points
        .iter()
        .map(|(_, v)| v.truncate_n(max_idx as isize))
        .collect();

    let a = projected_points[0];
    let b = projected_points[1];
    let c = projected_points[2];
    let n = (b - a).cross(c - a).normalize();
    let centroid = projected_points.iter().fold(Vector3::zero(), |i, j| i + j)
        / projected_points.len() as f32;

    let first = (a - centroid).normalize();
    let mut angles: Vec<_> = projected_points
        .iter()
        .enumerate()
        .map(|(i, v)| {
            let edge = (v - centroid).normalize();
            let mut angle = first.dot(edge).min(1.0).max(-1.0).acos();
            if n.dot(first.cross(edge)) < 0.0 {
                angle *= -1.0;
            }
            (angle, i)
        })
        .collect();
    angles.sort_by(|a, b| a.partial_cmp(b).unwrap());

    let mut sorted_points = Vec::new();
    for (_, i) in angles.iter() {
        sorted_points.push(points[*i]);
    }

    sorted_points
}

pub fn cell_120_simplices() -> (Vec<Vertex4>, Vec<u32>) {
    let mut colored_verts = Vec::new();
    let mut indices = Vec::new();

    let vertices = cell_120_verts();
    let faces = cell_120_faces();

    for (normal, face_indices) in cell_120_cells().iter() {
        use hsl::HSL;
        let (r, g, b) = HSL {
            h: 180.0 * (normal.z as f64 + rand::random::<f64>() * 5.0 - 2.5)
                % 360.0
                + 360.0,
            s: 1.0,
            l: 0.5 + rand::random::<f64>() * 0.1,
        }
        .to_rgb();
        let color = Vector4::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        );

        let apex = vertices[faces[face_indices[0]]];
        let apex_id = faces[face_indices[0]];
        let apex_colored_id = colored_verts.len();
        colored_verts.push(Vertex4 {
            position: apex,
            color,
        });

        for face_idx in face_indices.iter() {
            let face_vert_indices = &faces[*face_idx..face_idx + 5];
            if face_vert_indices.contains(&apex_id) {
                // skip
                continue;
            }

            // need to sort the points on a plane.
            let vertices: Vec<_> = face_vert_indices
                .iter()
                .map(|i| (*i, vertices[*i]))
                .collect();
            let sorted_vertices = sort_points_on_plane(normal, &vertices);
            let prev_colored_len = colored_verts.len();
            for (_, v) in sorted_vertices.iter() {
                colored_verts.push(Vertex4 {
                    position: *v,
                    color,
                });
            }
            for i in 1..sorted_vertices.len() - 1 {
                indices.push(apex_colored_id);
                indices.push(prev_colored_len);
                indices.push(prev_colored_len + i);
                indices.push(prev_colored_len + i + 1);
            }
        }
    }

    (colored_verts, indices.iter().map(|i| *i as u32).collect())
}

pub fn cell_600_simplices() -> (Vec<Vertex4>, Vec<u32>) {
    let mut colored_verts = Vec::new();
    let mut indices = Vec::new();

    let vertices = cell_600_verts();
    let faces = cell_600_faces();

    for (normal, face_indices) in cell_600_cells().iter() {
        use hsl::HSL;
        let (r, g, b) = HSL {
            h: 180.0 * (normal.z as f64 + rand::random::<f64>() * 10.0 - 5.0)
                % 360.0
                + 360.0,
            s: 1.0,
            l: 0.5 + rand::random::<f64>() * 0.1,
        }
        .to_rgb();
        let color = Vector4::new(
            r as f32 / 255.0,
            g as f32 / 255.0,
            b as f32 / 255.0,
            1.0,
        );

        let apex = vertices[faces[face_indices[0]]];
        let apex_id = faces[face_indices[0]];
        let apex_colored_id = colored_verts.len();
        colored_verts.push(Vertex4 {
            position: apex,
            color,
        });

        for face_idx in face_indices.iter() {
            let face_vert_indices = &faces[*face_idx..face_idx + 3];
            if face_vert_indices.contains(&apex_id) {
                // skip
                continue;
            }

            // need to sort the points on a plane.
            let vertices: Vec<_> = face_vert_indices
                .iter()
                .map(|i| (*i, vertices[*i]))
                .collect();
            let sorted_vertices = sort_points_on_plane(normal, &vertices);
            let prev_colored_len = colored_verts.len();
            for (_, v) in sorted_vertices.iter() {
                colored_verts.push(Vertex4 {
                    position: *v,
                    color,
                });
            }
            for i in 1..sorted_vertices.len() - 1 {
                indices.push(apex_colored_id);
                indices.push(prev_colored_len);
                indices.push(prev_colored_len + i);
                indices.push(prev_colored_len + i + 1);
            }
        }
    }

    (colored_verts, indices.iter().map(|i| *i as u32).collect())
}
