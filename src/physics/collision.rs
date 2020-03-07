use super::{Body, Collider};
use cgmath::{InnerSpace, Vector4};

pub struct Collision {
    pub impulse: Vector4<f32>,
    pub projection: Vector4<f32>,
}

pub fn collide(a: &Body, b: &Body) -> Option<Collision> {
    match (&a.collider, &b.collider) {
        (Collider::HalfPlane { .. }, Collider::Tesseract { half_width }) => {
            // just treat the half-plane as y = 0 for now.
            let normal = Vector4::unit_y();
            let min_y = b.pos.y - half_width;
            if min_y < 0.0 {
                let rel_vel = (b.vel - a.vel).dot(normal);
                if rel_vel > 0.0 {
                    return None;
                }
                let e = a.material.restitution.min(b.material.restitution);
                let inv_a_mass = if a.mass > 0.0 { 1.0 / a.mass } else { 0.0 };
                let inv_b_mass = if b.mass > 0.0 { 1.0 / b.mass } else { 0.0 };
                let impulse = -(1.0 + e) * rel_vel / (inv_a_mass + inv_b_mass);

                let penetration = -min_y;

                let slop_limit = 0.01f32;
                let slop_amount = 0.2f32;
                let projection = (penetration - slop_limit).max(0.0)
                    * slop_amount
                    / (inv_a_mass + inv_b_mass)
                    * normal;

                Some(Collision {
                    impulse: impulse * normal,
                    projection,
                })
            } else {
                None
            }
        }
        (Collider::Tesseract { .. }, Collider::HalfPlane { .. }) => {
            collide(b, a)
        }
        _ => unimplemented!(),
    }
}
