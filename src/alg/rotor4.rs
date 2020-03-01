use super::{Bivec4, Quadvec4, Vec4};
use cgmath::Matrix4;

#[derive(Debug, Clone)]
pub struct Rotor4 {
    pub s: f32,
    pub b: Bivec4,
    pub q: Quadvec4,
}

impl Rotor4 {
    pub fn identity() -> Self {
        Rotor4 {
            s: 1.,
            b: Bivec4::zero(),
            q: Quadvec4::zero(),
        }
    }

    pub fn new(s: f32, b: Bivec4, q: Quadvec4) -> Self {
        Self { s, b, q }
    }

    pub fn rotate(&self, v: &Vec4) -> Vec4 {
        // p = R v ~R. We do this in two steps:
        // Q = R v
        let (a_1, a_3) = self.b.mul_v(v);
        let b_3 = self.q.mul_v(v);
        let q_1 = self.s * v.clone() + a_1;
        let q_3 = a_3 + b_3;

        // p = Q ~R
        let b_rev = self.b.reverse();
        let p = self.s * q_1.clone()
            + q_1.left_contract_bv(&b_rev)
            + q_3.right_contract_bv(&b_rev)
            + q_3.mul_qv(&self.q);

        p
    }

    pub fn mul_bv(&self, c: &Bivec4) -> Rotor4 {
        let (a_0, a_2, a_4) = self.b.mul_bv(c);
        Self {
            s: a_0,
            b: self.s * c.clone() + a_2 + self.q.mul_bv(c),
            q: a_4,
        }
    }

    pub fn update(&mut self, delta: &Bivec4) {
        let delta_r = self.mul_bv(delta);
        self.s = self.s + -0.5 * delta_r.s;
        self.b = self.b.clone() + -0.5 * delta_r.b;
        self.q = self.q + -0.5 * delta_r.q;
        self.normalize();
    }

    pub fn normalize(&mut self) {
        let neg_xyzw = self.b.xy * self.b.zw - self.b.xz * self.b.zw
            + self.b.xw * self.b.yz;
        let pos_xyzw = self.s * self.q.xyzw;

        let neg_factor = neg_xyzw.abs().sqrt();
        let pos_factor = pos_xyzw.abs().sqrt();

        if neg_xyzw != 0.0
            && pos_xyzw != 0.0
            && neg_xyzw.signum() == pos_xyzw.signum()
        {
            self.s *= neg_factor;
            self.b = pos_factor * self.b.clone();
            self.q = neg_factor * self.q.clone();
        }

        let inverse_mag = 1.0 / self.mag();
        self.s *= inverse_mag;
        self.b = inverse_mag * self.b.clone();
        self.q = inverse_mag * self.q.clone();
    }

    pub fn mag(&self) -> f32 {
        let mag_sq = self.s * self.s
            + self.b.xy * self.b.xy
            + self.b.xz * self.b.xz
            + self.b.xw * self.b.xw
            + self.b.yw * self.b.yw
            + self.b.zw * self.b.zw
            + self.q.xyzw * self.q.xyzw;
        mag_sq.sqrt()
    }

    pub fn to_matrix(&self) -> Matrix4<f32> {
        let x = self.rotate(&Vec4 {
            x: 1.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        });
        let y = self.rotate(&Vec4 {
            x: 0.0,
            y: 1.0,
            z: 0.0,
            w: 0.0,
        });
        let z = self.rotate(&Vec4 {
            x: 0.0,
            y: 0.0,
            z: 1.0,
            w: 0.0,
        });
        let w = self.rotate(&Vec4 {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 1.0,
        });

        // attributes are not allowed on expressions apparently
        #[rustfmt::skip]
        return Matrix4::new(
            x.x, x.y, x.z, x.w,
            y.x, y.y, y.z, y.w,
            z.x, z.y, z.z, z.w,
            w.x, w.y, w.z, w.w,
        );
    }
}

impl Default for Rotor4 {
    fn default() -> Self {
        Self::identity()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::SquareMatrix;

    #[test]
    fn deteriminant_always_one() {
        let mut rotor = Rotor4::new(
            1.0,
            Bivec4::new(1.0, 0.0, 0.0, 0.0, 0.0, 1.0),
            Quadvec4 { xyzw: 1.0 },
        );
        rotor.normalize();

        println!(
            "{:?} {:?} {}",
            rotor,
            rotor.to_matrix(),
            rotor.to_matrix().determinant()
        );
    }

    #[test]
    fn what() {
        let mut rotor = Rotor4::identity();
        let bv = Bivec4::new(2.0, 0.0, 0.0, 0.0, 0.0, 2.0);
        rotor.update(&bv);

        println!("{:?}", rotor);
        println!("{:?}", rotor.mul_bv(&bv));
    }
}
