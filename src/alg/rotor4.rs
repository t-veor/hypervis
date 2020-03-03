use super::{Bivec4, Quadvec4, Vec4};
use cgmath::Matrix4;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy)]
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

    pub fn reverse(&self) -> Rotor4 {
        Rotor4::new(self.s, self.b.reverse(), self.q)
    }

    pub fn rotate(&self, v: &Vec4) -> Vec4 {
        // p = R v ~R. We do this in two steps:
        // Q = R v
        let (a_1, a_3) = self.b.mul_v(v);
        let b_3 = self.q.mul_v(v);
        let q_1 = self.s * *v + a_1;
        let q_3 = a_3 + b_3;

        // p = Q ~R
        let b_rev = self.b.reverse();
        let p = self.s * q_1
            + q_1.left_contract_bv(&b_rev)
            + q_3.right_contract_bv(&b_rev)
            + q_3.mul_qv(&self.q);

        p
    }

    pub fn mul_bv(&self, c: &Bivec4) -> Rotor4 {
        let (a_0, a_2, a_4) = self.b.mul_bv(c);
        Self {
            s: a_0,
            b: self.s * *c + a_2 + self.q.mul_bv(c),
            q: a_4,
        }
    }

    pub fn update(&mut self, delta: &Bivec4) {
        *self = *self * (-0.5 * *delta).exp();
        self.normalize();
    }

    pub fn normalize(&mut self) {
        // we decompose into two isoclinic rotations, which are each equivalent
        // to a quaternion. Each quaternion component is normalised, and then we
        // recover the original rotor

        let (mut r_plus, mut r_minus) = self.decompose();

        // get rid of the 1/2 (1 +- xyzw) components
        r_plus.s -= 0.5;
        r_minus.s -= 0.5;
        // we're going to overwrite the quadvector components since they should
        // be just +- the scalar components.

        let plus_mag = 2.0
            * (r_plus.s.powi(2)
                + r_plus.b.xy.powi(2)
                + r_plus.b.xz.powi(2)
                + r_plus.b.xw.powi(2))
            .sqrt();
        let minus_mag = 2.0
            * (r_minus.s.powi(2)
                + r_minus.b.xy.powi(2)
                + r_minus.b.xz.powi(2)
                + r_minus.b.xw.powi(2))
            .sqrt();

        if plus_mag > 0.0 {
            let inv_plus_mag = 1.0 / plus_mag;
            r_plus.s *= inv_plus_mag;
            r_plus.b = inv_plus_mag * r_plus.b;
            r_plus.q.xyzw = r_plus.s;

            // readd 1/2 (1 - xyzw)
            r_plus.s += 0.5;
            r_plus.q.xyzw -= 0.5;
        } else {
            // TODO:
            // unimplemented!("{:?} has zero magnitude!", r_plus);
            r_plus = Rotor4::identity();
        }

        if minus_mag > 0.0 {
            let inv_minus_mag = 1.0 / minus_mag;
            r_minus.s *= inv_minus_mag;
            r_minus.b = inv_minus_mag * r_minus.b;
            r_minus.q.xyzw = -r_minus.s;

            // readd 1/2 (1 + xyzw)
            r_minus.s += 0.5;
            r_minus.q.xyzw += 0.5;
        } else {
            // TODO
            // unimplemented!("{:?} has zero magnitude!", r_minus);
            r_minus = Rotor4::identity();
        }

        *self = r_plus * r_minus;
    }

    pub fn mag(&self) -> f32 {
        let mag_sq = self.s * self.s
            + self.b.xy * self.b.xy
            + self.b.xz * self.b.xz
            + self.b.xw * self.b.xw
            + self.b.yz * self.b.yz
            + self.b.yw * self.b.yw
            + self.b.zw * self.b.zw
            + self.q.xyzw * self.q.xyzw;
        mag_sq.sqrt()
    }

    pub fn weird_term(&self) -> f32 {
        -2.0 * self.b.xw * self.b.yz - 2.0 * self.b.xy * self.b.zw
            + 2.0 * self.b.xz * self.b.yw
            + 2.0 * self.q.xyzw * self.s
    }

    // Perform Cayley Factorisation to factorise the rotor into two pure
    // isoclinic rotations.
    pub fn decompose(&self) -> (Rotor4, Rotor4) {
        let pos_half_xyzw = Quadvec4::new(0.5);
        let neg_half_xyzw = Quadvec4::new(-0.5);

        let r_plus = Rotor4::new(
            0.5 + 0.5 * self.s + 0.5 * self.q.xyzw,
            0.5 * self.b + pos_half_xyzw.mul_bv(&self.b),
            0.5 * self.q + self.s * pos_half_xyzw + neg_half_xyzw,
        );

        let r_minus = Rotor4::new(
            0.5 + 0.5 * self.s - 0.5 * self.q.xyzw,
            0.5 * self.b + neg_half_xyzw.mul_bv(&self.b),
            0.5 * self.q + self.s * neg_half_xyzw + pos_half_xyzw,
        );

        (r_plus, r_minus)
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

impl Add<Rotor4> for Rotor4 {
    type Output = Rotor4;
    fn add(self, other: Rotor4) -> Rotor4 {
        Rotor4::new(self.s + other.s, self.b + other.b, self.q + other.q)
    }
}

impl Mul<Rotor4> for Rotor4 {
    type Output = Rotor4;
    fn mul(self, r_1: Rotor4) -> Rotor4 {
        let r_0 = self;
        let (a_0, a_2, a_4) = r_0.b.mul_bv(&r_1.b);
        Rotor4::new(
            r_0.s * r_1.s + a_0 + r_0.q.xyzw * r_1.q.xyzw,
            r_0.s * r_1.b
                + r_1.s * r_0.b
                + a_2
                + r_0.q.mul_bv(&r_1.b)
                + r_1.q.mul_bv(&r_0.b),
            r_0.s * r_1.q + r_1.s * r_0.q + a_4,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use cgmath::SquareMatrix;

    #[test]
    fn long_term_rotation_error() {
        let mut r = Rotor4::identity();
        for _ in 0..100000 {
            r.update(&Bivec4::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0));
        }
        println!(
            "{:?}\n{} {} {}",
            r,
            r.mag(),
            r.weird_term(),
            r.to_matrix().determinant()
        );
    }
}
