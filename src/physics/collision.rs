use super::{Body, CollisionManifold};
use crate::alg::Vec4;
use cgmath::{InnerSpace, Vector3, Vector4};

#[derive(Debug)]
pub struct ContactState {
    contact: Vector4<f32>,
    bias: f32,
    normal_mass: f32,
    normal_impulse: f32,
    tangent_mass: [f32; 3],
    tangent_impulse: [f32; 3],
}

pub struct CollisionConstraint {
    normal: Vector4<f32>,
    tangents: [Vector4<f32>; 3],
    contacts: Vec<ContactState>,
    mu: f32,
}

impl CollisionConstraint {
    pub fn new(
        manifold: CollisionManifold,
        a: &Body,
        mass_adjustment_a: f32,
        b: &Body,
        mass_adjustment_b: f32,
    ) -> Self {
        let CollisionManifold {
            normal,
            depth,
            contacts,
        } = manifold;

        let e = a.material.restitution.min(b.material.restitution);
        // TODO: move this into the Material struct
        let mu = 1.5;

        let tangents = compute_basis(normal);

        let contacts: Vec<_> = contacts
            .into_iter()
            .map(|contact| {
                let rel_vel = b.vel_at(contact) - a.vel_at(contact);
                let rel_vel_normal = rel_vel.dot(normal);

                let slop = 0.01;
                let baumgarte = 0.2;
                let bias = -baumgarte * 60.0 * (slop - depth).min(0.0)
                    + if rel_vel_normal < -1.0 {
                        -e * rel_vel_normal
                    } else {
                        0.0
                    };

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

                let inverse_mass_term =
                    |body: &Body,
                     normal: Vector4<f32>,
                     contact: Vector4<f32>| {
                        // n' = ~R n R
                        let body_normal = body.world_vec_to_body(normal);
                        let body_contact = body.world_pos_to_body(contact);

                        // n . (R x . I_b^-1(x /\ n') ~R)
                        normal.dot(
                            body.body_vec_to_world(
                                Vec4::from(body_contact)
                                    .left_contract_bv(
                                        &body.inverse_moment_of_inertia(
                                            &Vec4::from(body_contact)
                                                .wedge_v(&body_normal.into()),
                                        ),
                                    )
                                    .into(),
                            ),
                        )
                    };

                let inv_l_a =
                    mass_adjustment_a * inverse_mass_term(a, normal, contact);
                let inv_l_b =
                    mass_adjustment_b * inverse_mass_term(b, normal, contact);

                let normal_mass =
                    1.0 / (inv_a_mass + inv_b_mass + inv_l_a + inv_l_b);

                let mut tangent_mass = [0.0, 0.0, 0.0];
                for i in 0..3 {
                    let inv_l_t_a = mass_adjustment_a
                        * inverse_mass_term(a, tangents[i], contact);
                    let inv_l_t_b = mass_adjustment_b
                        * inverse_mass_term(b, tangents[i], contact);

                    tangent_mass[i] =
                        1.0 / (inv_a_mass + inv_b_mass + inv_l_t_a + inv_l_t_b);
                }

                ContactState {
                    contact,
                    bias,
                    normal_mass,
                    normal_impulse: 0.0,
                    tangent_mass,
                    tangent_impulse: [0.0, 0.0, 0.0],
                }
            })
            .collect();

        Self {
            normal,
            tangents,
            contacts,
            mu,
        }
    }

    pub fn solve(&mut self, a: &mut Body, b: &mut Body) {
        for contact_state in self.contacts.iter_mut() {
            let ContactState {
                contact,
                bias,
                normal_mass,
                ref mut normal_impulse,
                tangent_mass,
                ref mut tangent_impulse,
            } = *contact_state;

            let rel_vel = b.vel_at(contact) - a.vel_at(contact);

            // calculate friction impulse
            for i in 0..3 {
                let lambda = -rel_vel.dot(self.tangents[i]) * tangent_mass[i];
                let max_lambda = self.mu * *normal_impulse;
                let prev_impulse = tangent_impulse[i];
                tangent_impulse[i] =
                    (prev_impulse + lambda).min(max_lambda).max(-max_lambda);
                let impulse =
                    self.tangents[i] * (tangent_impulse[i] - prev_impulse);
                a.resolve_impulse(-impulse, contact);
                b.resolve_impulse(impulse, contact);
            }

            // calculate normal impulse
            let rel_vel = b.vel_at(contact) - a.vel_at(contact);
            let rel_vel_normal = rel_vel.dot(self.normal);
            let lambda = normal_mass * (-rel_vel_normal + bias);
            let prev_impulse = *normal_impulse;
            *normal_impulse = (prev_impulse + lambda).max(0.0);
            let impulse = self.normal * (*normal_impulse - prev_impulse);
            a.resolve_impulse(-impulse, contact);
            b.resolve_impulse(impulse, contact);
        }
    }
}

// Extension of https://box2d.org/posts/2014/02/computing-a-basis/ to 4 dimensions.
// (Refer to https://www.geometrictools.com/Documentation/OrthonormalSets.pdf)
fn compute_basis(a: Vector4<f32>) -> [Vector4<f32>; 3] {
    // If a is normalized, since 1 / 4 = 0.25 at least one component of a must
    // be >= sqrt(0.25)
    let (b, c) = if a.x.abs() >= 0.5 || a.y.abs() >= 0.5 {
        let b = Vector4::new(a.y, -a.x, 0.0, 0.0).normalize();
        let c = Vector3::new(a.x, a.y, a.z)
            .cross(Vector3::new(b.x, b.y, b.z))
            .normalize();
        (b, Vector4::new(c.x, c.y, c.z, 0.0))
    } else {
        let b = Vector4::new(0.0, 0.0, a.w, -a.z).normalize();
        let c = Vector3::new(a.y, a.z, a.w)
            .cross(Vector3::new(b.y, b.z, b.w))
            .normalize();
        (b, Vector4::new(0.0, c.x, c.y, c.z))
    };

    let d = crate::alg::triple_cross_product(a, b, c).normalize();
    [b, c, d]
}
#[cfg(test)]
mod test {
    use super::*;
    #[test]
    pub fn compute_basis_test() {
        let a = Vector4::new(1.0, 0.0, 0.0, 1.0).normalize();
        let [b, c, d] = dbg!(compute_basis(a));
        dbg!(a.dot(b), a.dot(c), a.dot(d), b.dot(c), b.dot(d), c.dot(d));
    }
}
