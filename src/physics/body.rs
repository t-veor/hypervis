use super::Collider;
use crate::alg::{Bivec4, Rotor4};
use cgmath::Vector4;

#[derive(Debug, Clone)]
pub struct Material {
    pub restitution: f32,
}

#[derive(Clone)]
pub struct Body {
    pub mass: f32,
    pub material: Material,
    pub stationary: bool,

    pub pos: Vector4<f32>,
    pub vel: Vector4<f32>,
    pub rotation: Rotor4,
    pub angular_vel: Bivec4,

    pub collider: Collider,
}

impl Body {
    pub fn apply_impulse(&mut self, impulse: Vector4<f32>) {
        if !self.stationary {
            self.vel += impulse / self.mass;
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
}
