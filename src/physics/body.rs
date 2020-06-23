use super::Collider;
use crate::alg::{Bivec4, Rotor4, Vec4};
use cgmath::{InnerSpace, Vector4, Zero};

#[derive(Debug, Clone)]
pub struct Material {
    pub restitution: f32,
}

#[derive(Debug, Clone)]
pub struct Velocity {
    pub linear: Vector4<f32>,
    pub angular: Bivec4,
}

impl Velocity {
    pub fn zero() -> Self {
        Self {
            linear: Vector4::zero(),
            angular: Bivec4::zero(),
        }
    }
}

#[derive(Debug, Clone)]
pub enum MomentInertia {
    Isotropic(f32),
    Principal(Bivec4),
}

#[derive(Clone)]
pub struct Body {
    pub mass: f32,
    pub moment_inertia: MomentInertia,
    pub material: Material,
    pub stationary: bool,

    pub pos: Vector4<f32>,
    pub rotation: Rotor4,

    pub vel: Velocity,

    pub collider: Collider,
}

impl Body {
    pub fn resolve_impulse(
        &mut self,
        impulse: Vector4<f32>,
        world_contact: Vector4<f32>,
    ) {
        if !self.stationary {
            let body_contact = self.world_pos_to_body(world_contact);
            let delta_angular_vel = self.inverse_moment_of_inertia(
                &Vec4::from(body_contact)
                    .wedge_v(&self.rotation.reverse().rotate(&impulse.into())),
            );

            self.vel.linear += impulse / self.mass;
            self.vel.angular = self.vel.angular + delta_angular_vel;
        }
    }

    pub fn step(&mut self, dt: f32) {
        if !self.stationary {
            // apply gravity
            self.vel.linear += Vector4::unit_y() * (-9.8 * dt);

            self.pos += self.vel.linear * dt;
            self.rotation.update(&(dt * self.vel.angular));
        }
    }

    pub fn inverse_moment_of_inertia(&self, body_bivec: &Bivec4) -> Bivec4 {
        match self.moment_inertia {
            MomentInertia::Isotropic(i) if i <= 0.0 => Bivec4::zero(),
            MomentInertia::Isotropic(i) => 1.0 / i * *body_bivec,
            MomentInertia::Principal(b) => Bivec4::new(
                body_bivec.xy / b.xy,
                body_bivec.xz / b.xz,
                body_bivec.xw / b.xw,
                body_bivec.yz / b.yz,
                body_bivec.yw / b.yw,
                body_bivec.zw / b.zw,
            ),
        }
    }

    pub fn vel_at(&self, world_pos: Vector4<f32>) -> Vector4<f32> {
        let body_pos = self.world_pos_to_body(world_pos);

        let rot_vel = self.body_vec_to_world(
            Vec4::from(body_pos)
                .left_contract_bv(&self.vel.angular)
                .into(),
        );

        self.vel.linear + rot_vel
    }
    pub fn ray_intersect(
        &self,
        start: Vector4<f32>,
        dir: Vector4<f32>,
    ) -> Option<f32> {
        let start = self.world_pos_to_body(start);
        let dir = self.world_vec_to_body(dir);

        match &self.collider {
            Collider::Mesh { mesh } => {
                let mut interval = (std::f32::NEG_INFINITY, std::f32::INFINITY);

                for cell in mesh.cells.iter() {
                    // grab a representative vertex on the cell
                    let v0 = mesh.vertices[mesh.edges
                        [mesh.faces[cell.faces[0]].edges[0]]
                        .hd_vertex];

                    let denom = dir.dot(cell.normal);
                    let lambda = (v0 - start).dot(cell.normal) / denom;

                    if denom < 0.0 {
                        interval.0 = interval.0.max(lambda);
                    } else {
                        interval.1 = interval.1.min(lambda);
                    }

                    if interval.1 < interval.0 {
                        return None;
                    }
                }

                Some(interval.0)
            }
            Collider::Sphere { radius } => {
                // Solve a quadratic equation!
                let a = dir.magnitude2();
                let b = 2.0 * start.dot(dir);
                let c = start.magnitude2() - radius * radius;

                let discriminant = b * b - 4.0 * a * c;
                if discriminant >= 0.0 {
                    Some((-b - discriminant.sqrt()) / (2.0 * a))
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn body_vec_to_world(&self, v: Vector4<f32>) -> Vector4<f32> {
        self.rotation.rotate(&v.into()).into()
    }

    pub fn world_vec_to_body(&self, v: Vector4<f32>) -> Vector4<f32> {
        self.rotation.reverse().rotate(&v.into()).into()
    }

    pub fn body_pos_to_world(&self, v: Vector4<f32>) -> Vector4<f32> {
        let rotated: Vector4<f32> = self.rotation.rotate(&v.into()).into();
        rotated + self.pos
    }

    pub fn world_pos_to_body(&self, v: Vector4<f32>) -> Vector4<f32> {
        self.rotation
            .reverse()
            .rotate(&(v - self.pos).into())
            .into()
    }
}
