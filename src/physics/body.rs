use super::{Collider, CollisionResponse};
use crate::alg::{Bivec4, Rotor4};
use cgmath::Vector4;

#[derive(Debug, Clone)]
pub struct Material {
    pub restitution: f32,
}

#[derive(Clone)]
pub struct Body {
    pub mass: f32,
    // for tesseracts it's sufficient to keep this as a scalar, but it really
    // should be a tensor of shape Bivec4 -> Bivec4
    pub moment_inertia_scalar: f32,
    pub material: Material,
    pub stationary: bool,

    pub pos: Vector4<f32>,
    pub vel: Vector4<f32>,
    pub rotation: Rotor4,
    pub angular_vel: Bivec4,

    pub collider: Collider,
}

impl Body {
    pub fn resolve_collision(
        &mut self,
        collision: &CollisionResponse,
        negate: bool,
    ) {
        if !self.stationary {
            let mut impulse = collision.impulse;
            let mut projection = collision.projection;
            let mut contact = &collision.contact_b;

            if negate {
                impulse = -impulse;
                projection = -projection;
                contact = &collision.contact_a;
            }

            let delta_angular_vel = self.inverse_moment_of_inertia(
                &self
                    .rotation
                    .reverse()
                    .rotate(&(*contact - self.pos).into())
                    .wedge_v(&self.rotation.reverse().rotate(&impulse.into())),
            );

            self.vel += impulse / self.mass;
            self.pos += projection / self.mass;
            self.angular_vel = self.angular_vel + delta_angular_vel;
        }
    }

    pub fn step(&mut self, dt: f32) {
        if !self.stationary {
            // apply gravity
            self.vel += Vector4::unit_y() * (-1.0 * dt);

            self.pos += self.vel * dt;
            self.rotation.update(&(dt * self.angular_vel));
        }
    }

    pub fn inverse_moment_of_inertia(&self, body_bivec: &Bivec4) -> Bivec4 {
        if self.moment_inertia_scalar <= 0.0 {
            return Bivec4::zero();
        }

        1.0 / self.moment_inertia_scalar * *body_bivec
    }
}
