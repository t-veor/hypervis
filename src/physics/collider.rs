use super::Body;
use crate::mesh::{ClipMesh, Mesh};

use cgmath::{
    Array, InnerSpace, Matrix3, SquareMatrix, Vector3, Vector4, Zero,
};
use smallvec::SmallVec;

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

pub fn detect_collisions(a: &Body, b: &Body) -> Option<CollisionManifold> {
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
            detect_collisions(b, a)
        }
        (Collider::Mesh { mesh: mesh_a }, Collider::Mesh { mesh: mesh_b }) => {
            if let Some(contact) = mesh_sat(a, mesh_a, b, mesh_b) {
                // dbg!(&contact);
                return Some(match contact {
                    ContactData::VertexCell(contact) => {
                        resolve_vertex_cell_contact(
                            a, mesh_a, b, mesh_b, contact,
                        )
                    }
                    ContactData::EdgeFace(contact) => {
                        resolve_edge_face_contact(a, mesh_a, b, mesh_b, contact)
                    }
                });
            }
            None
        }
        _ => None,
    }
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

fn mesh_sat(
    a: &Body,
    mesh_a: &Mesh,
    b: &Body,
    mesh_b: &Mesh,
) -> Option<ContactData> {
    let mut contact = None;
    let mut min_penetration = std::f32::INFINITY;

    // Check for edge-face intersections
    let mut check_edge_faces =
        |a: &Body, mesh_a: &Mesh, b: &Body, mesh_b: &Mesh, side: bool| {
            for edge in mesh_a.edges.iter() {
                // grab a representative vertex on the edge
                let v0 = mesh_a.vertices[edge.hd_vertex];
                // grab the edge vector
                let u = mesh_a.vertices[edge.tl_vertex] - v0;

                // loop through all the faces on b
                for face in mesh_b.faces.iter() {
                    // grab two edges on the face. Because of the way the face
                    // was generated, these edges are guaranteed to be
                    // non-parallel.
                    let (e0, e1) = (
                        &mesh_b.edges[face.edges[0]],
                        &mesh_b.edges[face.edges[1]],
                    );
                    // grab edge vectors
                    let v = a.world_vec_to_body(b.body_vec_to_world(
                        mesh_b.vertices[e0.tl_vertex]
                            - mesh_b.vertices[e0.hd_vertex],
                    ));
                    let w = a.world_vec_to_body(b.body_vec_to_world(
                        mesh_b.vertices[e1.tl_vertex]
                            - mesh_b.vertices[e1.hd_vertex],
                    ));

                    // grab the normal vector adjacent to all
                    let mut n =
                        crate::alg::triple_cross_product(u, v, w).normalize();
                    if !n.is_finite() {
                        continue;
                    }
                    let mut dist_a = n.dot(v0);
                    // ensure it's positive
                    if dist_a < 0.0 {
                        n = -n;
                        dist_a = -dist_a;
                    }
                    let mut min_dist_b = dist_a;

                    // loop through all the vertices on b
                    for v in mesh_b.vertices.iter() {
                        let dist_b =
                            a.world_pos_to_body(b.body_pos_to_world(*v)).dot(n);
                        if dist_b < min_dist_b {
                            min_dist_b = dist_b;
                        }
                    }

                    if min_dist_b < dist_a {
                        // Intersection along this axis
                        if dist_a - min_dist_b > min_penetration {
                            contact =
                                Some(ContactData::EdgeFace(EdgeFaceContact {
                                    side,
                                    k: a.body_pos_to_world(v0),
                                    t: a.body_vec_to_world(u),
                                    s: b.body_pos_to_world(
                                        mesh_b.vertices[e0.hd_vertex],
                                    ),
                                    u: a.body_vec_to_world(v),
                                    v: a.body_vec_to_world(w),
                                    normal: a.body_vec_to_world(n),
                                }));
                            min_penetration = dist_a - min_dist_b;
                        }
                    } else {
                        return true;
                    }
                }
            }
            return false;
        };

    /*
    if check_edge_faces(a, mesh_a, b, mesh_b, false) {
        return None;
    }

    if check_edge_faces(b, mesh_b, a, mesh_a, true) {
        return None;
    }
    */

    // Check for vertex-cell intersections
    let mut check_vertex_cell =
        |a: &Body, mesh_a: &Mesh, b: &Body, mesh_b: &Mesh, side: bool| {
            for (cell_idx, cell) in mesh_a.cells.iter().enumerate() {
                // grab a representative vertex on the cell to get the distance
                let v0 = mesh_a.vertices[mesh_a.edges
                    [mesh_a.faces[cell.faces[0]].edges[0]]
                    .hd_vertex];

                let dist_a = v0.dot(cell.normal);
                let mut min_dist_b = dist_a;
                let mut min_vertex_idx = 0;
                // loop through all the vertices on b
                for (vertex_idx, v) in mesh_b.vertices.iter().enumerate() {
                    let dist_b = a
                        .world_pos_to_body(b.body_pos_to_world(*v))
                        .dot(cell.normal);
                    if dist_b < min_dist_b {
                        min_dist_b = dist_b;
                        min_vertex_idx = vertex_idx;
                    }
                }

                if min_dist_b < dist_a {
                    // Intersection along this axis
                    if dist_a - min_dist_b < min_penetration {
                        contact =
                            Some(ContactData::VertexCell(VertexCellContact {
                                side,
                                vertex_idx: min_vertex_idx,
                                cell_idx,
                                normal: a.body_vec_to_world(cell.normal),
                            }));
                        min_penetration = dist_a - min_dist_b;
                    }
                } else {
                    // Found a separating axis!
                    return true;
                }
            }
            false
        };

    // Check all the surface normals of a's cells
    if check_vertex_cell(a, mesh_a, b, mesh_b, true) {
        return None;
    }

    // Check all the surface normals of b's cells
    if check_vertex_cell(b, mesh_b, a, mesh_a, false) {
        return None;
    }

    contact
}

fn resolve_vertex_cell_contact(
    a: &Body,
    mesh_a: &Mesh,
    b: &Body,
    mesh_b: &Mesh,
    contact: VertexCellContact,
) -> CollisionManifold {
    if !contact.side {
        // just swap the meshes around in the call
        let mut result = resolve_vertex_cell_contact(
            b,
            mesh_b,
            a,
            mesh_a,
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

    let reference_cell = &mesh_a.cells[contact.cell_idx];

    // Need to determine incident cell - find the cell with the least dot
    // product with the reference normal
    let mut min_dot_product = 1.0;
    let mut incident_cell_idx = 0;
    for cell_idx in mesh_b.vertex_data[contact.vertex_idx].cells.iter() {
        let candidate_cell = &mesh_b.cells[*cell_idx];
        let dot_product = b
            .body_vec_to_world(candidate_cell.normal)
            .dot(contact.normal);
        if dot_product < min_dot_product {
            min_dot_product = dot_product;
            incident_cell_idx = *cell_idx;
        }
    }

    // clip the incident cell against the adjacent cells of the reference cell
    let mut clipper = ClipMesh::from_cell(mesh_b, incident_cell_idx);
    let mut v0 = Vector4::zero();
    for face_idx in reference_cell.faces.iter() {
        let face = &mesh_a.faces[*face_idx];
        // grab a representative vertex
        v0 = mesh_a.vertices[mesh_a.edges[face.edges[0]].hd_vertex];

        let cell_idx = if face.hd_cell == contact.cell_idx {
            face.tl_cell
        } else {
            face.hd_cell
        };
        let clip_normal = b.world_vec_to_body(
            a.body_vec_to_world(-mesh_a.cells[cell_idx].normal),
        );
        let clip_distance =
            clip_normal.dot(b.world_pos_to_body(a.body_pos_to_world(v0)));

        clipper.clip_by(clip_normal, clip_distance);
    }
    let reference_dist = v0.dot(reference_cell.normal);

    // keep points that are below the reference plane
    let mut max_depth = 0f32;
    let contacts = clipper
        .to_vertices()
        .into_iter()
        .filter_map(|b_vec| {
            let world_vec = b.body_pos_to_world(b_vec);
            let a_vec = a.world_pos_to_body(world_vec);

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
        normal: a.body_vec_to_world(reference_cell.normal),
        depth: max_depth,
        contacts,
    }
}

fn resolve_edge_face_contact(
    a: &Body,
    mesh_a: &Mesh,
    b: &Body,
    mesh_b: &Mesh,
    contact: EdgeFaceContact,
) -> CollisionManifold {
    if !contact.side {
        // just swap the meshes around in the call
        let mut result = resolve_edge_face_contact(
            b,
            mesh_b,
            a,
            mesh_a,
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
    let x = mat.invert().unwrap() * y;

    let p1 = k + x.x * t;
    let p2 = s + k.y * u + k.z * v;
    let depth = (p1 - p2).magnitude();

    CollisionManifold {
        normal,
        depth,
        contacts: vec![p1 + p2 / 2.0],
    }
}
