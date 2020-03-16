use super::Mesh;
use crate::context::graphics::Vertex4;

use cgmath::Vector4;
use std::collections::HashMap;

#[derive(Debug)]
pub struct TetrahedronMesh {
    pub vertices: Vec<Vertex4>,
    pub indices: Vec<u32>,
}

fn get_face_vertex_indices(mesh: &Mesh, face_idx: usize) -> Vec<usize> {
    let mut vertex_indices = Vec::new();
    for edge_idx in mesh.faces[face_idx].edges.iter() {
        let edge = &mesh.edges[*edge_idx];
        if vertex_indices.contains(&edge.hd_vertex) {
            vertex_indices.push(edge.tl_vertex);
        } else {
            vertex_indices.push(edge.hd_vertex);
        }
    }

    vertex_indices
}

impl TetrahedronMesh {
    fn tetrahedralize_cell(
        mesh: &Mesh,
        cell_idx: usize,
        color: Vector4<f32>,
    ) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();
        let mut vertex_map = HashMap::new();

        let mut push_vertex = |vertex_idx: usize| {
            let colored_idx =
                *vertex_map.entry(vertex_idx).or_insert_with(|| {
                    let new_colored_vertex = Vertex4 {
                        position: mesh.vertices[vertex_idx],
                        color,
                    };
                    let new_colored_idx = vertices.len();
                    vertices.push(new_colored_vertex);
                    new_colored_idx
                });
            indices.push(colored_idx as u32);
        };

        let cell = &mesh.cells[cell_idx];

        // pick a point to be the apex from which all tetrahedra start from.
        let apex_idx = mesh.edges[mesh.faces[cell.faces[0]].edges[0]].hd_vertex;

        for face_idx in cell.faces.iter() {
            let vertex_indices = get_face_vertex_indices(mesh, *face_idx);
            if vertex_indices.contains(&apex_idx) {
                continue;
            }

            // because of the way faces were generated, we should be able to
            // assume all the vertices are already sorted in either clockwise or
            // anticlockwise order already
            for i in 1..vertex_indices.len() - 1 {
                push_vertex(apex_idx);
                push_vertex(vertex_indices[0]);
                push_vertex(vertex_indices[i]);
                push_vertex(vertex_indices[i + 1]);
            }
        }

        Self { vertices, indices }
    }

    fn append(
        &mut self,
        TetrahedronMesh { vertices, indices }: TetrahedronMesh,
    ) {
        let prev_len = self.vertices.len() as u32;
        self.vertices.extend(vertices);
        self.indices
            .extend(indices.into_iter().map(|i| i + prev_len));
    }

    pub fn from_mesh<F>(mesh: &Mesh, color_func: F) -> Self
    where
        F: Fn(Vector4<f32>) -> Vector4<f32>,
    {
        let mut result = TetrahedronMesh {
            vertices: Vec::new(),
            indices: Vec::new(),
        };

        for (cell_idx, cell) in mesh.cells.iter().enumerate() {
            let color = color_func(cell.normal);
            let cell_mesh = Self::tetrahedralize_cell(mesh, cell_idx, color);
            result.append(cell_mesh);
        }

        result
    }
}
