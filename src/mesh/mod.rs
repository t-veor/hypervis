mod tetrahedra;
mod todd_coxeter;

use cgmath::{InnerSpace, Vector4, Zero};
use smallvec::SmallVec;

pub use tetrahedra::*;

#[derive(Debug, Clone)]
pub struct VertexData {
    cells: SmallVec<[usize; 16]>,
}

#[derive(Debug, Clone)]
pub struct Edge {
    hd_vertex: usize,
    tl_vertex: usize,
    faces: SmallVec<[usize; 8]>,
}

#[derive(Debug, Clone)]
pub struct Face {
    hd_cell: usize,
    tl_cell: usize,
    edges: SmallVec<[usize; 8]>,
}

#[derive(Debug, Clone)]
pub struct Cell {
    normal: Vector4<f32>,
    faces: SmallVec<[usize; 16]>,
}

#[derive(Debug)]
pub struct Mesh {
    vertices: Vec<Vector4<f32>>,
    vertex_data: Vec<VertexData>,
    edges: Vec<Edge>,
    faces: Vec<Face>,
    cells: Vec<Cell>,
}

fn reflect(v: Vector4<f32>, mirror_normal: Vector4<f32>) -> Vector4<f32> {
    v - 2.0 * mirror_normal.dot(v) * mirror_normal
}

fn get_mirror_normals(symbol: &[usize; 3]) -> [Vector4<f32>; 4] {
    use std::f32::consts::PI;

    let mut mirror_normals = [Vector4::zero(); 4];

    mirror_normals[0] = Vector4::unit_x();

    // dot(N_0, N_1) = cos(pi / symbol[0])
    mirror_normals[1].x = (PI / symbol[0] as f32).cos();
    mirror_normals[1].y = (1.0 - mirror_normals[1].x.powi(2)).sqrt();

    // dot(N_0, N_2) = cos(pi / 2) = 0
    // dot(N_1, N_2) = cos(pi / symbol[1])
    mirror_normals[2].y = (PI / symbol[1] as f32).cos() / mirror_normals[1].y;
    mirror_normals[2].z = (1.0 - mirror_normals[2].y.powi(2)).sqrt();

    // dot(N_0, N_3) = cos(pi / 2) = 0
    // dot(N_1, N_3) = cos(pi / 2) = 0
    // dot(N_2, N_3) = cos(pi / symbol[2])
    mirror_normals[3].z = (PI / symbol[2] as f32).cos() / mirror_normals[2].z;
    mirror_normals[3].w = (1.0 - mirror_normals[3].z.powi(2)).sqrt();

    mirror_normals
}

impl Mesh {
    pub fn from_schlafli_symbol(symbol: &[usize; 3]) -> Self {
        // determine the mirror normals
        let mirror_normals = get_mirror_normals(symbol);

        // pick a v0 so that it's on planes 1, 2, and 3, but not on 0
        // set v_0.x = 1 arbitrarily
        let mut v0: Vector4<f32> = Vector4::unit_x();
        v0.y = -mirror_normals[1].x * v0.x / mirror_normals[1].y;
        v0.z = -mirror_normals[2].y * v0.y / mirror_normals[2].z;
        v0.w = -mirror_normals[3].z * v0.z / mirror_normals[3].w;
        v0 = v0.normalize();

        // setup for todd-coxeter
        let num_gens = 4;
        let relations: &[&[usize]] = &[
            &[0, 0],
            &[1, 1],
            &[2, 2],
            &[3, 3],
            &[0, 1].repeat(symbol[0]),
            &[1, 2].repeat(symbol[1]),
            &[2, 3].repeat(symbol[2]),
            &[0, 2].repeat(2),
            &[0, 3].repeat(2),
            &[1, 3].repeat(2),
        ];

        let vertex_table =
            todd_coxeter::coset_table(num_gens, relations, &[1, 2, 3]);
        let vertices =
            todd_coxeter::table_bfs_fold(&vertex_table, 0, v0, |v, mirror| {
                reflect(v, mirror_normals[mirror])
            });

        let edge_table =
            todd_coxeter::coset_table(num_gens, relations, &[0, 2, 3]);
        // the initial edge is guaranteed to be (0, 1)
        let e0 = (0, 1);
        let edge_tmp = todd_coxeter::table_bfs_fold(
            &edge_table,
            0,
            e0,
            |(v0, v1), mirror| {
                (vertex_table[v0][mirror], vertex_table[v1][mirror])
            },
        );

        let face_table =
            todd_coxeter::coset_table(num_gens, relations, &[0, 1, 3]);
        // the initial face is all the edges invariant under the rotation (0, 1)
        let f0 = {
            let mut edges: SmallVec<[usize; 8]> = SmallVec::new();
            let mut curr_edge = 0;
            loop {
                edges.push(curr_edge);
                curr_edge = edge_table[edge_table[curr_edge][0]][1];
                if curr_edge == 0 {
                    break;
                }
            }
            edges
        };
        let face_tmp =
            todd_coxeter::table_bfs_fold(&face_table, 0, f0, |f, mirror| {
                f.into_iter().map(|e| edge_table[e][mirror]).collect()
            });

        let cell_table =
            todd_coxeter::coset_table(num_gens, relations, &[0, 1, 2]);
        // The initial cell is invariant under mirrors 0, 1, and 2.
        // So, if we just apply mirrors 0, 1, and 2 to the inital face a
        // whole bunch of times, we should recover all the faces in the initial
        // cell
        let c0 = {
            // also pick a vector to be on planes 0, 1, and 2 - this will be the
            // normal vector of the cell
            // it turns out that this is the unit-w vector because of the way we
            // chose the mirror normals
            let normal = Vector4::unit_w();

            let mut faces: SmallVec<[usize; 16]> = SmallVec::new();
            faces.push(0);
            let mut i = 0;
            while i < faces.len() {
                let f = faces[i];
                for j in 0..3 {
                    let new_face = face_table[f][j];
                    if !faces.contains(&new_face) {
                        faces.push(new_face);
                    }
                }
                i += 1;
            }
            Cell { normal, faces }
        };
        let cells = todd_coxeter::table_bfs_fold(
            &cell_table,
            0,
            c0,
            |Cell { normal, faces }, mirror| Cell {
                normal: reflect(normal, mirror_normals[mirror]),
                faces: faces
                    .into_iter()
                    .map(|f| face_table[f][mirror])
                    .collect(),
            },
        );

        let mut edges: Vec<_> = edge_tmp
            .into_iter()
            .map(|(v0, v1)| Edge {
                hd_vertex: v0,
                tl_vertex: v1,
                faces: SmallVec::new(),
            })
            .collect();

        let mut faces: Vec<_> = face_tmp
            .into_iter()
            .map(|edges| Face {
                hd_cell: std::usize::MAX,
                tl_cell: std::usize::MAX,
                edges,
            })
            .collect();

        // populate faces for each edge
        for (i, face) in faces.iter().enumerate() {
            for j in face.edges.iter() {
                edges[*j].faces.push(i);
            }
        }

        // populate cells for each face
        for (i, cell) in cells.iter().enumerate() {
            for j in cell.faces.iter() {
                if faces[*j].hd_cell == std::usize::MAX {
                    faces[*j].hd_cell = i;
                } else {
                    faces[*j].tl_cell = i;
                }
            }
        }

        // populate cells for each vertex
        let mut vertex_data = vec![
            VertexData {
                cells: SmallVec::new()
            };
            vertices.len()
        ];
        for (cell_idx, cell) in cells.iter().enumerate() {
            for face_idx in cell.faces.iter() {
                for edge_idx in faces[*face_idx].edges.iter() {
                    let edge = &edges[*edge_idx];
                    let v0 = &mut vertex_data[edge.hd_vertex];
                    if !v0.cells.contains(&cell_idx) {
                        v0.cells.push(cell_idx);
                    }
                    let v1 = &mut vertex_data[edge.tl_vertex];
                    if !v1.cells.contains(&cell_idx) {
                        v1.cells.push(cell_idx);
                    }
                }
            }
        }

        Self {
            vertices,
            vertex_data,
            edges,
            faces,
            cells,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn schlafli() {
        dbg!(Mesh::from_schlafli_symbol(&[3, 3, 3]));
    }
}
