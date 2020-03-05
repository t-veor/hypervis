use super::{Body, Collider};
use cgmath::{InnerSpace, Vector4};

pub fn collide(a: &Body, b: &Body) -> Option<Vector4<f32>> {
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
                let impulse =
                    -(1.0 + e) * rel_vel / (1.0 / a.mass + 1.0 / b.mass);

                Some(normal * impulse)
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
