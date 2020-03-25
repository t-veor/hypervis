use super::{Body, CollisionManifold};
use crate::alg::Vec4;
use cgmath::{Array, InnerSpace, Vector4, Zero};

pub struct Impulse {
    pub impulse: Vector4<f32>,
    pub position: Vector4<f32>,
}

pub struct CollisionResponse {
    pub impulses: Vec<Impulse>,
    pub projection: Vector4<f32>,
}

pub fn calc_impulse(
    instance: CollisionManifold,
    a: &mut Body,
    mass_adjustment_a: f32,
    b: &mut Body,
    mass_adjustment_b: f32,
) -> CollisionResponse {
    let CollisionManifold {
        normal,
        depth,
        contacts,
    } = instance;

    let contact_len = contacts.len() as f32;
    for contact in contacts {
        let mass_adjustment_a = mass_adjustment_a * contact_len;
        let mass_adjustment_b = mass_adjustment_b * contact_len;

        let body_contact_a = a.world_pos_to_body(contact);
        let body_contact_b = b.world_pos_to_body(contact);

        let body_rot_vel_a =
            Vec4::from(body_contact_a).left_contract_bv(&a.angular_vel);
        let body_rot_vel_b =
            Vec4::from(body_contact_b).left_contract_bv(&b.angular_vel);
        let rot_vel_a: Vector4<f32> = a.rotation.rotate(&body_rot_vel_a).into();
        let rot_vel_b: Vector4<f32> = b.rotation.rotate(&body_rot_vel_b).into();

        let rel_vel = b.vel + rot_vel_b - a.vel - rot_vel_a;
        let rel_vel_normal = rel_vel.dot(normal);
        if rel_vel_normal > 0.0 {
            continue;
        }

        let e = a.material.restitution.min(b.material.restitution);

        let inv_a_mass = if a.mass > 0.0 {
            mass_adjustment_a / a.mass
        } else {
            0.0
        };
        let inv_b_mass = if b.mass > 0.0 {
            mass_adjustment_b / b.mass
        } else {
            0.0
        };

        let body_normal_a = a.world_vec_to_body(normal);
        let body_normal_b = b.world_vec_to_body(normal);

        let inv_l_a = mass_adjustment_a
            * normal.dot(
                a.body_vec_to_world(
                    Vec4::from(body_contact_a)
                        .left_contract_bv(
                            &a.inverse_moment_of_inertia(
                                &Vec4::from(body_contact_a)
                                    .wedge_v(&body_normal_a.into()),
                            ),
                        )
                        .into(),
                ),
            );
        let inv_l_b = mass_adjustment_b
            * normal.dot(
                b.body_vec_to_world(
                    Vec4::from(body_contact_b)
                        .left_contract_bv(
                            &b.inverse_moment_of_inertia(
                                &Vec4::from(body_contact_b)
                                    .wedge_v(&body_normal_b.into()),
                            ),
                        )
                        .into(),
                ),
            );

        let impulse = -(1.0 + e) * rel_vel_normal
            / (inv_a_mass + inv_b_mass + inv_l_a + inv_l_b);

        let mut tangent = (rel_vel - rel_vel.dot(normal) * normal).normalize();
        if !tangent.is_finite() {
            tangent = Vector4::zero();
        }

        let body_tangent_a = a.world_vec_to_body(tangent);
        let body_tangent_b = b.world_vec_to_body(tangent);

        let inv_l_tangent_a = mass_adjustment_a
            * tangent.dot(
                a.body_vec_to_world(
                    Vec4::from(body_contact_a)
                        .left_contract_bv(
                            &a.inverse_moment_of_inertia(
                                &Vec4::from(body_contact_a)
                                    .wedge_v(&body_tangent_a.into()),
                            ),
                        )
                        .into(),
                ),
            );
        let inv_l_tangent_b = mass_adjustment_b
            * tangent.dot(
                b.body_vec_to_world(
                    Vec4::from(body_contact_b)
                        .left_contract_bv(
                            &b.inverse_moment_of_inertia(
                                &Vec4::from(body_contact_b)
                                    .wedge_v(&body_tangent_b.into()),
                            ),
                        )
                        .into(),
                ),
            );

        let mut friction = -rel_vel.dot(tangent)
            / (inv_a_mass + inv_b_mass + inv_l_tangent_a + inv_l_tangent_b);
        let mu = 1.0;
        if friction.abs() > impulse * mu {
            friction = -impulse * mu;
        }

        let final_impulse = impulse * normal + friction * tangent;
        a.resolve_impulse(-final_impulse, contact);
        b.resolve_impulse(final_impulse, contact);
    }

    // Do some linear projection to stop bodies from just sinking into
    // each other
    let slop_limit = 0.01f32;
    let slop_amount = 0.8f32;
    let projection = (depth - slop_limit).max(0.0) * slop_amount * normal;

    CollisionResponse {
        impulses: Vec::new(),
        projection,
    }
}
