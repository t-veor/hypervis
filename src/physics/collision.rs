use super::{Body, CollisionInstance};
use crate::alg::Vec4;
use cgmath::{Array, InnerSpace, Vector4, Zero};

pub struct CollisionResponse {
    pub impulse: Vector4<f32>,
    pub projection: Vector4<f32>,
}

pub fn calc_impulse(
    instance: &CollisionInstance,
    a: &Body,
    mass_adjustment_a: f32,
    b: &Body,
    mass_adjustment_b: f32,
) -> Option<CollisionResponse> {
    let CollisionInstance {
        normal,
        depth,
        body_contact_a,
        body_contact_b,
    } = *instance;

    let body_rot_vel_a =
        Vec4::from(body_contact_a).left_contract_bv(&a.angular_vel);
    let body_rot_vel_b =
        Vec4::from(body_contact_b).left_contract_bv(&b.angular_vel);
    let rot_vel_a: Vector4<f32> = a.rotation.rotate(&body_rot_vel_a).into();
    let rot_vel_b: Vector4<f32> = b.rotation.rotate(&body_rot_vel_b).into();

    let rel_vel = b.vel + rot_vel_b - a.vel - rot_vel_a;
    let rel_vel_normal = rel_vel.dot(normal);
    if rel_vel_normal > 0.0 {
        return None;
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

    let body_normal_a = a.rotation.reverse().rotate(&normal.into());
    let body_normal_b = b.rotation.reverse().rotate(&normal.into());

    let inv_l_a = mass_adjustment_a
        * normal.dot(
            a.rotation
                .rotate(&Vec4::from(body_contact_a).left_contract_bv(
                    &a.inverse_moment_of_inertia(
                        &Vec4::from(body_contact_a).wedge_v(&body_normal_a),
                    ),
                ))
                .into(),
        );
    let inv_l_b = mass_adjustment_b
        * normal.dot(
            b.rotation
                .rotate(&Vec4::from(body_contact_b).left_contract_bv(
                    &b.inverse_moment_of_inertia(
                        &Vec4::from(body_contact_b).wedge_v(&body_normal_b),
                    ),
                ))
                .into(),
        );

    let impulse = -(1.0 + e) * rel_vel_normal
        / (inv_a_mass + inv_b_mass + inv_l_a + inv_l_b);

    let mut tangent = (rel_vel - rel_vel.dot(normal) * normal).normalize();
    if !tangent.is_finite() {
        tangent = Vector4::zero();
    }

    let body_tangent_a = a.rotation.reverse().rotate(&tangent.into());
    let body_tangent_b = b.rotation.reverse().rotate(&tangent.into());

    let inv_l_tangent_a = mass_adjustment_a
        * tangent.dot(
            a.rotation
                .rotate(&Vec4::from(body_contact_a).left_contract_bv(
                    &a.inverse_moment_of_inertia(
                        &Vec4::from(body_contact_a).wedge_v(&body_tangent_a),
                    ),
                ))
                .into(),
        );
    let inv_l_tangent_b = mass_adjustment_b
        * tangent.dot(
            b.rotation
                .rotate(&Vec4::from(body_contact_b).left_contract_bv(
                    &b.inverse_moment_of_inertia(
                        &Vec4::from(body_contact_b).wedge_v(&body_tangent_b),
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
    let projection = (depth - slop_limit).max(0.0) * slop_amount * normal;

    Some(CollisionResponse {
        impulse: (impulse * normal + friction * tangent),
        projection,
    })
}
