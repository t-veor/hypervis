use super::{Body, Collider};
use cgmath::{Array, InnerSpace, Vector4, Zero};

pub struct CollisionInstance {
    pub normal: Vector4<f32>,
    pub depth: f32,
    pub contact_a: Vector4<f32>,
    pub contact_b: Vector4<f32>,
}

pub struct CollisionResponse {
    pub impulse: Vector4<f32>,
    pub projection: Vector4<f32>,
    pub contact_a: Vector4<f32>,
    pub contact_b: Vector4<f32>,
}

pub fn detect_collision(a: &Body, b: &Body) -> Option<CollisionInstance> {
    match (&a.collider, &b.collider) {
        (
            Collider::HalfSpace { normal },
            Collider::Tesseract { half_width },
        ) => {
            let plane_distance = a.pos.dot(*normal);

            let mut contact_points = Vec::new();
            let mut tesseract_distance = 0.0;
            for mut i in 0..16 {
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
                if contact_points.len() == 0 {
                    contact_points.push(pos);
                    tesseract_distance = distance;
                } else if (distance - tesseract_distance).abs() < 0.0001 {
                    contact_points.push(pos)
                } else if distance < tesseract_distance {
                    contact_points.clear();
                    contact_points.push(pos);
                    tesseract_distance = distance;
                }
            }

            let contact_point = contact_points
                .iter()
                .fold(Vector4::zero(), |sum, i| sum + i)
                / contact_points.len() as f32;

            if plane_distance - tesseract_distance > 0.0 {
                Some(CollisionInstance {
                    normal: *normal,
                    depth: plane_distance - tesseract_distance,
                    contact_a: a.pos,
                    contact_b: contact_point,
                })
            } else {
                None
            }
        }
        (Collider::Tesseract { .. }, Collider::HalfSpace { .. }) => {
            // Just call this again with the arguments swapped
            detect_collision(b, a)
        }
        _ => unimplemented!(),
    }
}

pub fn collide(a: &Body, b: &Body) -> Option<CollisionResponse> {
    detect_collision(a, b).and_then(
        |CollisionInstance {
             normal,
             depth,
             contact_a,
             contact_b,
         }| {
            let body_contact_a =
                a.rotation.reverse().rotate(&(contact_a - a.pos).into());
            let body_contact_b =
                b.rotation.reverse().rotate(&(contact_b - b.pos).into());
            let body_rot_vel_a =
                body_contact_a.left_contract_bv(&a.angular_vel);
            let body_rot_vel_b =
                body_contact_b.left_contract_bv(&b.angular_vel);
            let rot_vel_a: Vector4<f32> =
                a.rotation.rotate(&body_rot_vel_a).into();
            let rot_vel_b: Vector4<f32> =
                b.rotation.rotate(&body_rot_vel_b).into();

            let rel_vel = b.vel + rot_vel_b - a.vel - rot_vel_a;
            let rel_vel_normal = rel_vel.dot(normal);
            if rel_vel_normal > 0.0 {
                return None;
            }

            let e = a.material.restitution.min(b.material.restitution);

            let inv_a_mass = if a.mass > 0.0 { 1.0 / a.mass } else { 0.0 };
            let inv_b_mass = if b.mass > 0.0 { 1.0 / b.mass } else { 0.0 };

            let body_normal_a = a.rotation.reverse().rotate(&normal.into());
            let body_normal_b = b.rotation.reverse().rotate(&normal.into());

            let inv_l_a = normal.dot(
                a.rotation
                    .rotate(&body_contact_a.left_contract_bv(
                        &a.inverse_moment_of_inertia(
                            &body_contact_a.wedge_v(&body_normal_a),
                        ),
                    ))
                    .into(),
            );
            let inv_l_b = normal.dot(
                b.rotation
                    .rotate(&body_contact_b.left_contract_bv(
                        &b.inverse_moment_of_inertia(
                            &body_contact_b.wedge_v(&body_normal_b),
                        ),
                    ))
                    .into(),
            );

            let impulse = -(1.0 + e) * rel_vel_normal
                / (inv_a_mass + inv_b_mass + inv_l_a + inv_l_b);

            let mut tangent =
                (rel_vel - rel_vel.dot(normal) * normal).normalize();
            if !tangent.is_finite() {
                tangent = Vector4::zero();
            }

            let body_tangent_a = a.rotation.reverse().rotate(&tangent.into());
            let body_tangent_b = b.rotation.reverse().rotate(&tangent.into());

            let inv_l_tangent_a = tangent.dot(
                a.rotation
                    .rotate(&body_contact_a.left_contract_bv(
                        &a.inverse_moment_of_inertia(
                            &body_contact_a.wedge_v(&body_tangent_a),
                        ),
                    ))
                    .into(),
            );
            let inv_l_tangent_b = tangent.dot(
                b.rotation
                    .rotate(&body_contact_b.left_contract_bv(
                        &b.inverse_moment_of_inertia(
                            &body_contact_b.wedge_v(&body_tangent_b),
                        ),
                    ))
                    .into(),
            );

            let mut friction = -rel_vel.dot(tangent)
                / (inv_a_mass + inv_b_mass + inv_l_tangent_a + inv_l_tangent_b);
            let mu = 0.4;
            if friction.abs() > impulse * mu {
                friction = -impulse * mu;
            }

            // Do some linear projection to stop bodies from just sinking into
            // each other
            let slop_limit = 0.01f32;
            let slop_amount = 0.2f32;
            let projection =
                (depth - slop_limit).max(0.0) * slop_amount * normal;

            Some(CollisionResponse {
                impulse: impulse * normal + friction * tangent,
                projection,
                contact_a,
                contact_b,
            })
        },
    )
}
