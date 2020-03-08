use super::Body;
use cgmath::{InnerSpace, Vector4};

#[derive(Clone)]
pub enum Collider {
    HalfSpace { normal: Vector4<f32> },
    Tesseract { half_width: f32 },
}

pub struct CollisionInstance {
    pub normal: Vector4<f32>,
    pub depth: f32,
    pub body_contact_a: Vector4<f32>,
    pub body_contact_b: Vector4<f32>,
}

pub fn detect_collisions(a: &Body, b: &Body) -> Vec<CollisionInstance> {
    match (&a.collider, &b.collider) {
        (
            Collider::HalfSpace { normal },
            Collider::Tesseract { half_width },
        ) => {
            let plane_distance = a.pos.dot(*normal);

            (0..16)
                .filter_map(|mut i| {
                    let mut position = [*half_width; 4];
                    for j in 0..4 {
                        if i % 2 == 0 {
                            position[j] *= -1.0;
                        }
                        i /= 2;
                    }
                    let pos = b
                        .rotation
                        .rotate(&mint::Vector4::from_slice(&position).into());
                    let pos = Vector4::new(pos.x, pos.y, pos.z, pos.w) + b.pos;

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
        (Collider::Tesseract { .. }, Collider::HalfSpace { .. }) => {
            // Just call this again with the arguments swapped
            detect_collisions(b, a)
        }
        _ => unimplemented!(),
    }
}
