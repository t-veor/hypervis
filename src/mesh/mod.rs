mod todd_coxeter;

use cgmath::{InnerSpace, Vector4, Zero};
use smallvec::SmallVec;

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
    pub fn from_schlafli_symbol(symbol: &[usize; 3]) {
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
        let vertices: Vec<_> = todd_coxeter::coset_table_bfs(&vertex_table, 0)
            .iter()
            .map(|mirrors| {
                mirrors
                    .iter()
                    .fold(v0, |v, i| reflect(v, mirror_normals[*i]))
            })
            .collect();

        let edge_table =
            todd_coxeter::coset_table(num_gens, relations, &[0, 2, 3]);
        // the initial edge is guaranteed to be (0, 1)
        let e0 = (0, 1);
        let edge_tmp: Vec<_> = todd_coxeter::coset_table_bfs(&edge_table, 0)
            .iter()
            .map(|mirrors| {
                mirrors.iter().fold(e0, |(v0, v1), i| {
                    (vertex_table[v0][*i], vertex_table[v1][*i])
                })
            })
            .collect();

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
        let face_tmp: Vec<_> = todd_coxeter::coset_table_bfs(&face_table, 0)
            .iter()
            .map(|mirrors| {
                mirrors.iter().fold(f0.clone(), |f, i| {
                    f.into_iter().map(|e| edge_table[e][*i]).collect()
                })
            })
            .collect();

        let cell_table =
            todd_coxeter::coset_table(num_gens, relations, &[0, 1, 2]);
        // The initial cell is invariant under mirrors 0, 1, and 2.
        // So, if we just apply mirrors 0, 1, and 2 to the inital face a
        // whole bunch of times, we should recover all the faces in the initial
        // cell
        let c0 = {
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
            faces
        };
        let cell_tmp: Vec<_> = todd_coxeter::coset_table_bfs(&cell_table, 0)
            .iter()
            .map(|mirrors| {
                mirrors.iter().fold(c0.clone(), |c, i| {
                    c.into_iter().map(|e| face_table[e][*i]).collect()
                })
            })
            .collect();
        dbg!(cell_tmp);

        // dbg!(vertices);
        // dbg!(edge_tmp);
        // dbg!(face_tmp);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn schlafli() {
        Mesh::from_schlafli_symbol(&[3, 3, 3]);
    }
}
