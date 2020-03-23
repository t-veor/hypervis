use super::Body;
use crate::mesh::Mesh;

use cgmath::{InnerSpace, Vector4};

#[derive(Clone)]
pub enum Collider {
    HalfSpace { normal: Vector4<f32> },
    Mesh { mesh: Mesh },
}

pub struct CollisionInstance {
    pub normal: Vector4<f32>,
    pub depth: f32,
    pub body_contact_a: Vector4<f32>,
    pub body_contact_b: Vector4<f32>,
}

pub fn detect_collisions(a: &Body, b: &Body) -> Vec<CollisionInstance> {
    match (&a.collider, &b.collider) {
        (Collider::HalfSpace { normal }, Collider::Mesh { mesh }) => {
            let plane_distance = a.pos.dot(*normal);

            mesh.vertices
                .iter()
                .filter_map(|position| {
                    let pos = b.body_to_world(*position);

                    let distance = pos.dot(*normal);

                    if plane_distance - distance > 0.0 {
                        Some(CollisionInstance {
                            normal: *normal,
                            depth: plane_distance - distance,
                            body_contact_a: a.world_to_body(a.pos),
                            body_contact_b: b.world_to_body(pos),
                        })
                    } else {
                        None
                    }
                })
                .collect()
        }
        (Collider::Mesh { .. }, Collider::HalfSpace { .. }) => {
            // Just call this again with the arguments swapped
            detect_collisions(b, a)
        }
        _ => Vec::new(),
    }
}
