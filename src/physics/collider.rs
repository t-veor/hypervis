use super::Body;
use crate::mesh::{ClipMesh, Mesh};
use crate::util::EPSILON;
use crate::world::ObjectKey;

use cgmath::{
    Array, InnerSpace, Matrix3, SquareMatrix, Vector3, Vector4, Zero,
};

#[derive(Clone)]
pub enum Collider {
    HalfSpace { normal: Vector4<f32> },
    Mesh { mesh: Mesh },
}

#[derive(Debug)]
pub struct CollisionManifold {
    pub normal: Vector4<f32>,
    pub depth: f32,
    pub contacts: Vec<Vector4<f32>>,
}

#[derive(Copy, Clone)]
pub struct MeshRef<'a> {
    pub body: &'a Body,
    pub mesh: &'a Mesh,
}

#[derive(Debug)]
struct VertexCellContact {
    // if true indicates that the vertex is on body b but the cell is on body a
    side: bool,
    vertex_idx: usize,
    cell_idx: usize,
    normal: Vector4<f32>,
}

#[derive(Debug)]
struct EdgeFaceContact {
    // if true indicates that the edge is on body b but the face is on body a
    side: bool,
    k: Vector4<f32>,
    t: Vector4<f32>,
    s: Vector4<f32>,
    u: Vector4<f32>,
    v: Vector4<f32>,
    normal: Vector4<f32>,
}

#[derive(Debug)]
enum ContactData {
    VertexCell(VertexCellContact),
    EdgeFace(EdgeFaceContact),
}

#[derive(Copy, Clone, Debug)]
enum ContactAxis {
    VertexCell {
        side: bool,
        cell_idx: usize,
    },
    EdgeFace {
        side: bool,
        edge_idx: usize,
        face_idx: usize,
    },
}

#[derive(Debug)]
enum AxisResult {
    Intersection {
        penetration: f32,
        contact: ContactData,
    },
    NotValidAxis,
    LargerPenetration,
    NoIntersection {
        normal: Vector4<f32>,
    },
}

pub struct CollisionDetection {
    sat_cache: lru::LruCache<(ObjectKey, ObjectKey), Vector4<f32>>,
}

impl CollisionDetection {
    pub fn new() -> Self {
        Self {
            sat_cache: lru::LruCache::new(1000),
        }
    }

    pub fn detect_collisions(
        &mut self,
        key: (ObjectKey, ObjectKey),
        a: &Body,
        b: &Body,
    ) -> Option<CollisionManifold> {
        match (&a.collider, &b.collider) {
            (Collider::HalfSpace { normal }, Collider::Mesh { mesh }) => {
                let plane_distance = a.pos.dot(*normal);
                let mut max_depth = 0.0;

                let contacts: Vec<_> = mesh
                    .vertices
                    .iter()
                    .filter_map(|position| {
                        let pos = b.body_pos_to_world(*position);

                        let distance = pos.dot(*normal);

                        let depth = plane_distance - distance;
                        if depth > 0.0 {
                            if depth > max_depth {
                                max_depth = depth;
                            }
                            Some(pos)
                        } else {
                            None
                        }
                    })
                    .collect();

                if contacts.len() > 0 {
                    Some(CollisionManifold {
                        normal: *normal,
                        depth: max_depth,
                        contacts,
                    })
                } else {
                    None
                }
            }
            (Collider::Mesh { .. }, Collider::HalfSpace { .. }) => {
                // Just call this again with the arguments swapped
                self.detect_collisions((key.1, key.0), b, a)
            }
            (
                Collider::Mesh { mesh: mesh_a },
                Collider::Mesh { mesh: mesh_b },
            ) => {
                let a = MeshRef {
                    body: a,
                    mesh: mesh_a,
                };
                let b = MeshRef {
                    body: b,
                    mesh: mesh_b,
                };
                if let Some(contact) = self.mesh_sat(key, a, b) {
                    // dbg!(&contact);
                    return Some(match contact {
                        ContactData::VertexCell(contact) => {
                            resolve_vertex_cell_contact(a, b, contact)
                        }
                        ContactData::EdgeFace(contact) => {
                            resolve_edge_face_contact(a, b, contact)
                        }
                    });
                }
                None
            }
            _ => None,
        }
    }

    fn mesh_sat(
        &mut self,
        key: (ObjectKey, ObjectKey),
        a: MeshRef,
        b: MeshRef,
    ) -> Option<ContactData> {
        // Bounding hypersphere check
        if (a.body.pos - b.body.pos).magnitude2()
            > (a.mesh.radius + b.mesh.radius).powi(2)
        {
            return None;
        }

        let mut min_penetration = std::f32::INFINITY;
        let mut curr_contact = None;

        let mut edge_cells_cache = None;

        if let Some(axis) = self.sat_cache.get(&key) {
            let axis = *axis;
            if !self.fast_check_axis(a, b, axis) {
                return None;
            }
            // If we got here then the cache entry is no longer useful.
            self.sat_cache.pop(&key);
        }

        macro_rules! axis_check {
            ($axis: expr) => {
                match Self::check_axis(
                    a,
                    b,
                    $axis,
                    min_penetration,
                    &mut edge_cells_cache,
                ) {
                    AxisResult::Intersection {
                        penetration,
                        contact,
                    } => {
                        min_penetration = penetration;
                        curr_contact = Some(contact);
                    }
                    AxisResult::NoIntersection { normal } => {
                        self.sat_cache.put(key, normal);
                        return None;
                    }
                    _ => (),
                }
            };
        }

        for cell_idx in 0..a.mesh.cells.len() {
            let axis = ContactAxis::VertexCell {
                side: true,
                cell_idx,
            };
            axis_check!(axis);
        }

        for cell_idx in 0..b.mesh.cells.len() {
            let axis = ContactAxis::VertexCell {
                side: false,
                cell_idx,
            };
            axis_check!(axis);
        }

        // I'm not fully convinced that we need proper edge-face collision
        // detection. In the case that the SAT algorithm overestimates because
        // it does not consider an edge-face, the contact pruning algorithm
        // should prune the contact down to nothing. Additionally, edge-face
        // collision detection is _really_ slow...
        /*
        for edge_idx in 0..a.mesh.edges.len() {
            edge_cells_cache = None;
            for face_idx in 0..b.mesh.faces.len() {
                let axis = ContactAxis::EdgeFace {
                    side: false,
                    edge_idx,
                    face_idx,
                };
                axis_check!(axis);
            }
        }

        for edge_idx in 0..b.mesh.edges.len() {
            edge_cells_cache = None;
            for face_idx in 0..a.mesh.faces.len() {
                let axis = ContactAxis::EdgeFace {
                    side: true,
                    edge_idx,
                    face_idx,
                };
                axis_check!(axis);
            }
        }
        */

        curr_contact
    }

    fn axis_span(&self, a: MeshRef, normal: Vector4<f32>) -> (f32, f32) {
        let mut min = std::f32::NEG_INFINITY;
        let mut max = std::f32::INFINITY;

        for v in a.mesh.vertices.iter() {
            let d = a.body.body_pos_to_world(*v).dot(normal);
            min = min.min(d);
            max = max.max(d);
        }

        (min, max)
    }

    fn fast_check_axis(
        &self,
        a: MeshRef,
        b: MeshRef,
        normal: Vector4<f32>,
    ) -> bool {
        let a_range = self.axis_span(a, normal);
        let b_range = self.axis_span(b, normal);

        a_range.0 <= b_range.1 && b_range.0 <= a_range.1
    }

    fn check_axis(
        a: MeshRef,
        b: MeshRef,
        axis: ContactAxis,
        min_penetration: f32,
        edge_cells_ref: &mut Option<Vec<Vector4<f32>>>,
    ) -> AxisResult {
        match axis {
            ContactAxis::VertexCell { cell_idx, side } => {
                let (a, b) = if side { (a, b) } else { (b, a) };
                Self::check_vertex_cell(a, b, cell_idx, side, min_penetration)
            }
            ContactAxis::EdgeFace {
                edge_idx,
                face_idx,
                side,
            } => {
                let (a, b) = if side { (b, a) } else { (a, b) };
                Self::check_edge_face(
                    a,
                    b,
                    edge_idx,
                    face_idx,
                    side,
                    min_penetration,
                    edge_cells_ref,
                )
            }
        }
    }

    fn check_edge_face(
        a: MeshRef,
        b: MeshRef,
        edge_idx: usize,
        face_idx: usize,
        side: bool,
        min_penetration: f32,
        edge_cells_ref: &mut Option<Vec<Vector4<f32>>>,
    ) -> AxisResult {
        let edge = &a.mesh.edges[edge_idx];
        let face = &b.mesh.faces[face_idx];

        let edge_cells = match edge_cells_ref {
            Some(cs) => cs,
            None => {
                let mut cells = Vec::new();
                if let Some(face_idx) = edge.faces.first() {
                    let face = &a.mesh.faces[*face_idx];
                    cells.push(face.hd_cell);
                    cells.push(face.tl_cell);

                    for face_idx in edge.faces.iter().skip(1) {
                        let face = &a.mesh.faces[*face_idx];
                        if !cells.contains(&face.hd_cell) {
                            cells.push(face.hd_cell);
                        } else if !cells.contains(&face.tl_cell) {
                            cells.push(face.tl_cell);
                        }
                    }
                };
                let edge_cells =
                    cells.into_iter().map(|i| a.mesh.cells[i].normal).collect();
                *edge_cells_ref = Some(edge_cells);
                edge_cells_ref.as_ref().unwrap()
            }
        };

        // grab a representative vertex on the edge
        let v0 = a.mesh.vertices[edge.hd_vertex];
        // grab the edge vector
        let u = a.mesh.vertices[edge.tl_vertex] - v0;

        let c0 = a.body.world_vec_to_body(
            b.body.body_vec_to_world(b.mesh.cells[face.hd_cell].normal),
        );
        let c1 = a.body.world_vec_to_body(
            b.body.body_vec_to_world(b.mesh.cells[face.tl_cell].normal),
        );

        if !minkowski_edge_face_check(edge_cells, (-c0, -c1)) {
            return AxisResult::NotValidAxis;
        }

        // grab two edges on the face. Because of the way the face
        // was generated, these edges are guaranteed to be
        // non-parallel.
        let (e0, e1) =
            (&b.mesh.edges[face.edges[0]], &b.mesh.edges[face.edges[1]]);
        // grab edge vectors
        let v = a.body.world_vec_to_body(b.body.body_vec_to_world(
            b.mesh.vertices[e0.tl_vertex] - b.mesh.vertices[e0.hd_vertex],
        ));
        let w = a.body.world_vec_to_body(b.body.body_vec_to_world(
            b.mesh.vertices[e1.tl_vertex] - b.mesh.vertices[e1.hd_vertex],
        ));

        // grab a vector on the face also
        let v1 = a.body.world_pos_to_body(
            b.body.body_pos_to_world(b.mesh.vertices[e0.hd_vertex]),
        );

        // grab the normal vector adjacent to all
        let mut n = crate::alg::triple_cross_product(u, v, w).normalize();
        if !n.is_finite() {
            return AxisResult::NotValidAxis;
        }
        // ensure that n points from a to b
        let mut dist_a = n.dot(v0);
        if dist_a < 0.0 {
            n = -n;
            dist_a = -dist_a;
        }

        let dist_b = n.dot(v1);

        if dist_b < dist_a {
            if dist_a - dist_b < min_penetration {
                // Intersection along this axis
                AxisResult::Intersection {
                    penetration: dist_a - dist_b,
                    contact: ContactData::EdgeFace(EdgeFaceContact {
                        side,
                        k: a.body.body_pos_to_world(v0),
                        t: a.body.body_vec_to_world(u),
                        s: b.body
                            .body_pos_to_world(b.mesh.vertices[e0.hd_vertex]),
                        u: a.body.body_vec_to_world(v),
                        v: a.body.body_vec_to_world(w),
                        normal: a.body.body_vec_to_world(n),
                    }),
                }
            } else {
                AxisResult::LargerPenetration
            }
        } else {
            AxisResult::NoIntersection {
                normal: a.body.body_vec_to_world(n),
            }
        }
    }

    fn check_vertex_cell(
        a: MeshRef,
        b: MeshRef,
        cell_idx: usize,
        side: bool,
        min_penetration: f32,
    ) -> AxisResult {
        let cell = &a.mesh.cells[cell_idx];

        // grab a representative vertex on the cell to get the distance
        let v0 = a.mesh.vertices
            [a.mesh.edges[a.mesh.faces[cell.faces[0]].edges[0]].hd_vertex];

        let dist_a = v0.dot(cell.normal);
        let mut min_dist_b = dist_a;
        let mut min_vertex_idx = 0;
        // loop through all the vertices on b
        for (vertex_idx, v) in b.mesh.vertices.iter().enumerate() {
            let dist_b = a
                .body
                .world_pos_to_body(b.body.body_pos_to_world(*v))
                .dot(cell.normal);
            if dist_b < min_dist_b {
                min_dist_b = dist_b;
                min_vertex_idx = vertex_idx;
            }
        }

        if min_dist_b < dist_a {
            // Intersection along this axis
            if dist_a - min_dist_b < min_penetration {
                AxisResult::Intersection {
                    penetration: dist_a - min_dist_b,
                    contact: ContactData::VertexCell(VertexCellContact {
                        side,
                        vertex_idx: min_vertex_idx,
                        cell_idx,
                        normal: a.body.body_vec_to_world(cell.normal),
                    }),
                }
            } else {
                AxisResult::LargerPenetration
            }
        } else {
            // Found a separating axis!
            AxisResult::NoIntersection {
                normal: a.body.body_vec_to_world(cell.normal),
            }
        }
    }
}

fn resolve_vertex_cell_contact(
    a: MeshRef,
    b: MeshRef,
    contact: VertexCellContact,
) -> CollisionManifold {
    if !contact.side {
        // just swap the meshes around in the call
        let mut result = resolve_vertex_cell_contact(
            b,
            a,
            VertexCellContact {
                side: true,
                ..contact
            },
        );
        // flip the normal as the collision resolution code expects the normal
        // to be oriented in a certain way
        result.normal *= -1.0;
        return result;
    }

    let reference_cell = &a.mesh.cells[contact.cell_idx];

    // Need to determine incident cell - find the cell with the least dot
    // product with the reference normal
    let mut min_dot_product = 1.0;
    let mut incident_cell_idx = 0;
    for cell_idx in b.mesh.vertex_data[contact.vertex_idx].cells.iter() {
        let candidate_cell = &b.mesh.cells[*cell_idx];
        let dot_product = b
            .body
            .body_vec_to_world(candidate_cell.normal)
            .dot(contact.normal);
        if dot_product < min_dot_product {
            min_dot_product = dot_product;
            incident_cell_idx = *cell_idx;
        }
    }

    // clip the incident cell against the adjacent cells of the reference cell
    let mut clipper = ClipMesh::from_cell(b.mesh, incident_cell_idx);
    let mut v0 = Vector4::zero();
    for face_idx in reference_cell.faces.iter() {
        let face = &a.mesh.faces[*face_idx];
        // grab a representative vertex
        v0 = a.mesh.vertices[a.mesh.edges[face.edges[0]].hd_vertex];

        let cell_idx = if face.hd_cell == contact.cell_idx {
            face.tl_cell
        } else {
            face.hd_cell
        };
        let clip_normal = b.body.world_vec_to_body(
            a.body.body_vec_to_world(-a.mesh.cells[cell_idx].normal),
        );
        let clip_distance = clip_normal
            .dot(b.body.world_pos_to_body(a.body.body_pos_to_world(v0)));

        clipper.clip_by(clip_normal, clip_distance);
    }
    let reference_dist = v0.dot(reference_cell.normal);

    // keep points that are below the reference plane
    let mut max_depth = 0f32;
    let contacts = clipper
        .to_vertices()
        .into_iter()
        .filter_map(|b_vec| {
            let world_vec = b.body.body_pos_to_world(b_vec);
            let a_vec = a.body.world_pos_to_body(world_vec);

            let dist = a_vec.dot(reference_cell.normal);
            if dist < reference_dist {
                max_depth = max_depth.max(reference_dist - dist);
                Some(world_vec)
            } else {
                None
            }
        })
        .collect();

    CollisionManifold {
        normal: a.body.body_vec_to_world(reference_cell.normal),
        depth: max_depth,
        contacts,
    }
}

fn resolve_edge_face_contact(
    a: MeshRef,
    b: MeshRef,
    contact: EdgeFaceContact,
) -> CollisionManifold {
    if contact.side {
        // just swap the meshes around in the call
        let mut result = resolve_edge_face_contact(
            b,
            a,
            EdgeFaceContact {
                side: false,
                ..contact
            },
        );
        // flip the normal as the collision resolution code expects the normal
        // to be oriented in a certain way
        result.normal *= -1.0;
        return result;
    }

    let EdgeFaceContact {
        k,
        t,
        s,
        u,
        v,
        normal,
        ..
    } = contact;

    // Now we gotta solve an equation in three variables to get the closest point
    // form the matrix
    #[rustfmt::skip]
    let mat = Matrix3::new(
        t.dot(t), -t.dot(u), -t.dot(v),
        -t.dot(u), u.dot(u), u.dot(v),
        -t.dot(v), u.dot(v), v.dot(v),
    );
    // dbg!(mat);
    // dbg!(mat.determinant());
    let y = Vector3::new(-(k - s).dot(t), (k - s).dot(u), (k - s).dot(v));
    let x = match mat.invert() {
        Some(m) => m,
        None => {
            // This shouldn't really happen, but as a failsafe let's return an
            // empty contact
            return CollisionManifold {
                normal,
                depth: 0.0,
                contacts: Vec::new(),
            };
        }
    } * y;

    let p1 = k + x.x * t;
    let p2 = s + x.y * u + x.z * v;
    let depth = (p1 - p2).magnitude();

    CollisionManifold {
        normal,
        depth,
        contacts: vec![(p1 + p2) / 2.0],
    }
}

fn minkowski_edge_face_check(
    edge_cells: &Vec<Vector4<f32>>,
    face_cells: (Vector4<f32>, Vector4<f32>),
) -> bool {
    // grab the normal corresponding to the great sphere the edge lies in
    let normal = if let &[a, b, c, ..] = &edge_cells[..] {
        crate::alg::triple_cross_product(a, b, c)
    } else {
        return false;
    };

    // intersect the plane defined by the face with the hyperplane, giving us a
    // line
    let (u, v) = face_cells;
    let factor = -v.dot(normal) / u.dot(normal);
    if !factor.is_finite() {
        return false;
    }
    let t = u * factor + v;
    // intersect t with the great sphere, giving us two points
    let s0 = t.normalize();
    let s1 = -s0;

    // check that either s0 or s1 are inside the great arc
    let s = {
        let target_angle = u.dot(v).acos();
        let s0_u = s0.dot(u).acos();
        let s0_v = s0.dot(v).acos();
        let s1_u = s1.dot(u).acos();
        let s1_v = s1.dot(v).acos();

        if (target_angle - s0_u - s0_v).abs() < EPSILON {
            s0
        } else if (target_angle - s1_u - s1_v).abs() < EPSILON {
            s1
        } else {
            // neither s0 nor s1 are in the arc
            return false;
        }
    };

    // now check if s is actually in the spherical polygon corresponding to the edge
    for i in 0..edge_cells.len() {
        let (u, v, w) = (
            edge_cells[i],
            edge_cells[(i + 1) % edge_cells.len()],
            edge_cells[(i + 2) % edge_cells.len()],
        );
        let n = crate::alg::triple_cross_product(u, v, normal);
        // check that s is on the same side of the sphere as another point in the polygon, w
        if s.dot(n) * w.dot(n) < 0.0 {
            return false;
        }
    }

    // s is indeed an intersection!
    true
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::alg::Bivec4;
    use crate::physics::Material;
    use crate::physics::Velocity;

    #[test]
    pub fn edge_edge_separation() {
        /*
        let tess = Mesh::from_schlafli_symbol(&[4, 3, 3]);

        let tess_a = Body {
            mass: 1.0,
            moment_inertia_scalar: 1.0 / 6.0,
            material: Material { restitution: 0.4 },
            stationary: false,
            pos: Vector4::new(0.0, 0.0, 0.0, 0.0),
            rotation: Bivec4::new(
                0.0,
                std::f32::consts::FRAC_PI_8,
                0.0,
                0.0,
                0.0,
                0.0,
            )
            .exp(),
            vel: Velocity::zero(),
            collider: Collider::Mesh { mesh: tess.clone() },
        };

        let tess_b = Body {
            pos: Vector4::new(std::f32::consts::SQRT_2 - 0.1, 0.0, 0.0, 0.0),
            rotation: Bivec4::new(
                std::f32::consts::FRAC_PI_8,
                0.0,
                0.0,
                0.0,
                0.0,
                0.0,
            )
            .exp(),
            ..tess_a.clone()
        };

        dbg!(detect_collisions(&tess_a, &tess_b));
        */
    }
}
