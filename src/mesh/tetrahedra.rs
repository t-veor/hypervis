use super::Mesh;
use crate::context::graphics::Vertex4;

use cgmath::{InnerSpace, Vector4};
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

    pub fn subdivide(&self, frequency: usize) -> Self {
        let mut vertices = Vec::new();
        let mut indices = Vec::new();

        for tetrahedron in self.indices.chunks_exact(4) {
            if let &[a, b, c, d] = tetrahedron {
                let a = self.vertices[a as usize];
                let b = self.vertices[b as usize];
                let c = self.vertices[c as usize];
                let d = self.vertices[d as usize];

                let mut mapped_indices: Vec<(usize, usize, usize)> = Vec::new();

                for n in 0..frequency {
                    // loop through all the possible vertices on this layer
                    for i in 0..(n + 1) {
                        for j in 0..(n - i + 1) {
                            let k = n - i - j;
                            // insert a tetrahedron based at this vertex
                            // x, xi, xj, xk
                            mapped_indices.extend(&[
                                (i, j, k),
                                (i + 1, j, k),
                                (i, j + 1, k),
                                (i, j, k + 1),
                            ]);

                            if n < frequency - 1 {
                                // insert an octahedron here as well
                                mapped_indices.extend(&[
                                    // xi, xj, xk, xik
                                    (i + 1, j, k),
                                    (i, j + 1, k),
                                    (i, j, k + 1),
                                    (i + 1, j, k + 1),
                                    // xi, xj, xij, xik
                                    (i + 1, j, k),
                                    (i, j + 1, k),
                                    (i + 1, j + 1, k),
                                    (i + 1, j, k + 1),
                                    // xj, xk, xik, xjk
                                    (i, j + 1, k),
                                    (i, j, k + 1),
                                    (i + 1, j, k + 1),
                                    (i, j + 1, k + 1),
                                    // xj, xij, xik, xjk
                                    (i, j + 1, k),
                                    (i + 1, j + 1, k),
                                    (i + 1, j, k + 1),
                                    (i, j + 1, k + 1),
                                ]);
                            }

                            if n < frequency - 2 {
                                mapped_indices.extend(&[
                                    // xij, xik, xjk, xijk
                                    (i + 1, j + 1, k),
                                    (i + 1, j, k + 1),
                                    (i, j + 1, k + 1),
                                    (i + 1, j + 1, k + 1),
                                ]);
                            }
                        }
                    }
                }

                let mut vertex_map = HashMap::new();
                for coords in mapped_indices {
                    let index =
                        *vertex_map.entry(coords).or_insert_with(|| {
                            let index = vertices.len();

                            let (i, j, k) = coords;
                            let s = i as f32 / frequency as f32;
                            let t = j as f32 / frequency as f32;
                            let u = k as f32 / frequency as f32;
                            let r = 1.0 - s - t - u;

                            let position = a.position * r
                                + b.position * s
                                + c.position * t
                                + d.position * u;
                            let color = a.color * r
                                + b.color * s
                                + c.color * t
                                + d.color * u;

                            vertices.push(Vertex4 { position, color });

                            index
                        });
                    indices.push(index as u32);
                }
            }
        }

        Self { vertices, indices }
    }

    pub fn make_geodesic(&self, frequency: usize, radius: f32) -> Self {
        let mut mesh = self.subdivide(frequency);
        mesh.vertices
            .iter_mut()
            .for_each(|v| v.position = v.position.normalize() * radius);
        mesh
    }
}
