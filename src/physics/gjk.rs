use std::collections::HashMap;

use super::{CollisionManifold, MeshRef, SphereRef};
use crate::alg::triple_cross_product;
use crate::mesh::Mesh;
use crate::util::{NotNaN, EPSILON};
use cgmath::{InnerSpace, Matrix2, SquareMatrix, Vector2, Vector4, Zero};

pub trait Support {
    fn support(&self, direction: Vector4<f32>) -> Vector4<f32>;
}

impl<'a> Support for MeshRef<'a> {
    fn support(&self, direction: Vector4<f32>) -> Vector4<f32> {
        let body_d = self.body.world_vec_to_body(direction);
        self.body.body_pos_to_world(
            *self
                .mesh
                .vertices
                .iter()
                .max_by_key(|v| NotNaN::new(v.dot(body_d)).unwrap())
                .unwrap(),
        )
    }
}

impl<'a> Support for SphereRef<'a> {
    fn support(&self, direction: Vector4<f32>) -> Vector4<f32> {
        direction.normalize() * self.radius + self.body.pos
    }
}

#[derive(Debug, Clone, Copy)]
pub struct CSO<A, B>
where
    A: Support,
    B: Support,
{
    a: A,
    b: B,
}

impl<A, B> Support for CSO<A, B>
where
    A: Support,
    B: Support,
{
    fn support(&self, direction: Vector4<f32>) -> Vector4<f32> {
        self.a.support(direction) - self.b.support(-direction)
    }
}

fn support(m: MeshRef, direction: Vector4<f32>) -> Vector4<f32> {
    let body_d = m.body.world_vec_to_body(direction);
    m.body.body_pos_to_world(
        *m.mesh
            .vertices
            .iter()
            .max_by_key(|v| NotNaN::new(v.dot(body_d)).unwrap())
            .unwrap(),
    )
}

struct Simplex {
    vertices: [Vector4<f32>; 5],
    length: usize,
}

impl Simplex {
    fn new(initial_point: Vector4<f32>) -> Self {
        let mut vertices = [Vector4::zero(); 5];
        vertices[0] = initial_point;
        Self {
            vertices,
            length: 1,
        }
    }

    fn push(&mut self, point: Vector4<f32>) {
        if self.length >= 5 {
            panic!("Simplex is already full!");
        }

        self.vertices[self.length] = point;
        self.length += 1;
    }

    fn remove(&mut self, index: usize) {
        if index >= self.length {
            panic!("Invalid simplex index provided to remove_at");
        }
        if self.length == 1 {
            panic!("Simplex cannot have no points!");
        }

        for i in (index + 1)..self.length {
            self.vertices[i - 1] = self.vertices[i];
        }
        self.length -= 1;
    }

    // If self contains the origin, returns false. Otherwise, updates self to be
    // the closest simplex on self to the origin, and sets d to be a direction
    // towards the origin normal to the updated simplex.
    fn nearest_simplex(&mut self, direction: &mut Vector4<f32>) -> bool {
        match self.length {
            1 => {
                // single point case, set direction to -a and return false
                let a = self.vertices[0];
                *direction = -a;
                false
            }
            2 => {
                // line case, return a direction perpendicular
                let (a, b) = (self.vertices[0], self.vertices[1]);
                let ab = b - a;
                let lambda = -a.dot(ab) / ab.magnitude2();
                // lambda is now such that a + lambda * (b - a) is the point on
                // the defined by a and b closest to the origin.
                *direction = -a - lambda * ab;
                false
            }
            3 => {
                // triangle case, return a direction perpendicular
                let (a, b, c) =
                    (self.vertices[0], self.vertices[1], self.vertices[2]);
                let (ab, ac) = (b - a, c - a);

                // Equation of two variables, we're just going to use cgmath to
                // solve it
                #[rustfmt::skip]
                let mat = Matrix2::new(
                    ab.magnitude2(),      ab.dot(ac),
                         ab.dot(ac), ac.magnitude2(),
                );
                let y = Vector2::new(-a.dot(ab), -a.dot(ac));
                let x = mat.invert().unwrap() * y;

                let (lambda, mu) = (x.x, x.y);

                *direction = -a - lambda * ab - mu * ac;

                false
            }
            4 => {
                // tetrahedron case, return a direction perpendicular
                let a = self.vertices[0];
                let b = self.vertices[1];
                let c = self.vertices[2];
                let d = self.vertices[3];
                let (ab, ac, ad) = (b - a, c - a, d - a);

                // We can use the triple cross product to just grab a normal to
                // the tetrahedron
                *direction = triple_cross_product(ab, ac, ad);
                // check that the direction is pointing opposite a
                if a.dot(*direction) > 0.0 {
                    *direction = -*direction;
                }

                false
            }
            5 => {
                // Now we have a full 5-cell as our simplex. To check if the
                // origin is inside our simplex now, we simply need to perform
                // 5 halfspace tests.

                // We can actually skip one of the halfspace tests - because we
                // know that on the last iteration the direction was set to the
                // normal of the tetrahedron abcd, the origin must be in the
                // halfspace defined by the tetrahedron abcd.

                for i in 0..4 {
                    // tetrahedron is the 5 vertices without the ith vertex
                    let mut j = 0;
                    let mut tetrahedron = [Vector4::zero(); 4];
                    for k in 0..5 {
                        if k != i {
                            tetrahedron[j] = self.vertices[k];
                            j += 1;
                        }
                    }

                    let a = tetrahedron[0];
                    let b = tetrahedron[1];
                    let c = tetrahedron[2];
                    let d = tetrahedron[3];
                    // e is the last vertex not in the tetrahedron
                    let e = self.vertices[i];

                    let (ab, ac, ad) = (b - a, c - a, d - a);
                    let normal = triple_cross_product(ab, ac, ad);

                    // the origin has to be on the same side as e to pass the
                    // halfspace test!
                    let ao_dist = -a.dot(normal);
                    if ao_dist * (e - a).dot(normal) < 0.0 {
                        // failed the halfspace test, so we know e is on the
                        // opposite side of the tetrahedron to the origin.
                        // We can then remove e from the simplex and set
                        // direction to the normal pointing towards the origin.
                        self.remove(i);
                        if ao_dist > 0.0 {
                            // normal is pointing towards origin
                            *direction = normal;
                        } else {
                            // normal pointing away from origin
                            *direction = -normal;
                        }
                        return false;
                    }
                }

                // If we reach here we've passed all the halfspace tests, so
                // the tetrahedron does indeed contain the origin!
                true
            }
            _ => unreachable!("invalid simplex!"),
        }
    }
}

fn gjk_intersection<S>(
    object: S,
    initial_direction: Vector4<f32>,
) -> Option<Simplex>
where
    S: Support,
{
    let mut a = object.support(initial_direction);
    let mut s = Simplex::new(a);
    let mut d = -a;

    loop {
        a = object.support(d);
        if a.dot(d) < 0.0 {
            return None;
        }
        s.push(a);
        if s.nearest_simplex(&mut d) {
            return Some(s);
        }
    }
}

struct EPCell {
    faces: [[usize; 3]; 4],
    normal: Vector4<f32>,
    distance: f32,
}

impl EPCell {
    fn from_faces(
        vertices: &Vec<Vector4<f32>>,
        faces: [[usize; 3]; 4],
    ) -> Self {
        let mut unique_vertices = [0; 4];
        let mut unique_vertices_count = 0;
        'outer: for face in faces.iter() {
            for vertex_idx in face.iter() {
                if !&unique_vertices[0..unique_vertices_count]
                    .contains(vertex_idx)
                {
                    unique_vertices[unique_vertices_count] = *vertex_idx;
                    unique_vertices_count += 1;

                    if unique_vertices_count >= 4 {
                        break 'outer;
                    }
                }
            }
        }

        let a = vertices[unique_vertices[0]];
        let b = vertices[unique_vertices[1]];
        let c = vertices[unique_vertices[2]];
        let d = vertices[unique_vertices[3]];

        let mut normal = triple_cross_product(b - a, c - a, d - a);
        let mut distance = normal.dot(a);
        if distance < 0.0 {
            normal = -normal;
            distance = -distance;
        }

        Self {
            faces,
            normal,
            distance,
        }
    }
}

struct ExpandingPolytope {
    vertices: Vec<Vector4<f32>>,
    cells: Vec<EPCell>,
}

impl ExpandingPolytope {
    fn from_simplex(simplex: Simplex) -> Self {
        assert!(simplex.length == 5);
        let vertices = simplex.vertices.to_vec();

        let mut faces = Vec::new();
        faces.reserve(10);
        for i in 0..5 {
            for j in (i + 1)..5 {
                for k in (j + 1)..5 {
                    faces.push([i, j, k]);
                }
            }
        }

        let mut cells = Vec::new();
        cells.reserve(5);
        for i in 0..5 {
            let mut cell_faces = [[0, 0, 0]; 4];
            let mut cell_face_count = 0;
            for face in faces.iter() {
                if !face.contains(&i) {
                    cell_faces[cell_face_count] = *face;
                    cell_face_count += 1;

                    if cell_face_count >= 4 {
                        break;
                    }
                }
            }

            cells.push(EPCell::from_faces(&vertices, cell_faces));
        }

        Self { vertices, cells }
    }

    fn expand(&mut self, extend_point: Vector4<f32>) {
        let mut removed_faces = Vec::new();

        let cells = &mut self.cells;
        let vertices = &self.vertices;

        cells.retain(|cell| {
            let v0 = vertices[cell.faces[0][0]];
            if cell.normal.dot(extend_point - v0) > 0.0 {
                true
            } else {
                // Before returning, we need to add all the faces to the removed
                // faces
                for face in cell.faces.iter() {
                    match removed_faces.iter().position(|f| f == face) {
                        Some(i) => {
                            // found a duplicate, remove it from the removed
                            // faces
                            removed_faces.remove(i);
                        }
                        None => removed_faces.push(*face),
                    }
                }
                false
            }
        });

        // push the new vertex
        let vertex_idx = self.vertices.len();
        self.vertices.push(extend_point);

        // now add a new cell for each face
        for face in removed_faces {
            let mut cell_faces = [
                [face[0], face[1], vertex_idx],
                [face[0], face[2], vertex_idx],
                [face[1], face[2], vertex_idx],
                face,
            ];
            for f in cell_faces.iter_mut() {
                f.sort_unstable();
            }

            self.cells
                .push(EPCell::from_faces(&self.vertices, cell_faces));
        }
    }
}

fn epa<S>(object: S, simplex: Simplex) -> Vector4<f32>
where
    S: Support,
{
    let mut polytope = ExpandingPolytope::from_simplex(simplex);

    loop {
        let min_cell = polytope
            .cells
            .iter()
            .min_by_key(|i| NotNaN::new(i.distance).unwrap())
            .unwrap();
        let normal = min_cell.normal;
        let extend_point = object.support(normal);

        if extend_point.dot(normal) - min_cell.distance > EPSILON {
            polytope.expand(extend_point);
        } else {
            return normal;
        }
    }
}
