use super::{Quadvec4, Rotor4, Trivec4, Vec4};
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy)]
pub struct Bivec4 {
    pub xy: f32,
    pub xz: f32,
    pub xw: f32,
    pub yz: f32,
    pub yw: f32,
    pub zw: f32,
}

impl Bivec4 {
    pub fn new(xy: f32, xz: f32, xw: f32, yz: f32, yw: f32, zw: f32) -> Self {
        Self {
            xy,
            xz,
            xw,
            yz,
            yw,
            zw,
        }
    }

    pub fn zero() -> Self {
        Self::new(0.0, 0.0, 0.0, 0.0, 0.0, 0.0)
    }

    pub fn reverse(&self) -> Self {
        Self::new(-self.xy, -self.xz, -self.xw, -self.yz, -self.yw, -self.zw)
    }

    pub fn mul_v(&self, v: &Vec4) -> (Vec4, Trivec4) {
        let b = self;
        (
            Vec4 {
                x: b.xw * v.w + b.xy * v.y + b.xz * v.z,
                y: -b.xy * v.x + b.yw * v.w + b.yz * v.z,
                z: -b.xz * v.x - b.yz * v.y + b.zw * v.w,
                w: -b.xw * v.x - b.yw * v.y - b.zw * v.z,
            },
            Trivec4 {
                xyz: b.xy * v.z - b.xz * v.y + b.yz * v.x,
                xyw: -b.xw * v.y + b.xy * v.w + b.yw * v.x,
                xzw: -b.xw * v.z + b.xz * v.w + b.zw * v.x,
                yzw: -b.yw * v.z + b.yz * v.w + b.zw * v.y,
            },
        )
    }

    #[rustfmt::skip]
    pub fn mul_bv(&self, c: &Bivec4) -> (f32, Bivec4, Quadvec4) {
        let b = self;

        let s =
            - b.xy * c.xy
            - b.xz * c.xz
            - b.xw * c.xw
            - b.yz * c.yz
            - b.yw * c.yw
            - b.zw * c.zw;

        let d = Bivec4 {
            xy: - b.xw * c.yw - b.xz * c.yz + b.yw * c.xw + b.yz * c.xz,
            xz: - b.xw * c.zw + b.xy * c.yz - b.yz * c.xy + b.zw * c.xw,
            xw:   b.xy * c.yw + b.xz * c.zw - b.yw * c.xy - b.zw * c.xz,
            yz: - b.xy * c.xz + b.xz * c.xy - b.yw * c.zw + b.zw * c.yw,
            yw:   b.xw * c.xy - b.xy * c.xw + b.yz * c.zw - b.zw * c.yz,
            zw:   b.xw * c.xz - b.xz * c.xw + b.yw * c.yz - b.yz * c.yw,
        };

        let q = Quadvec4 {
            xyzw:
                  b.xw * c.yz
                + b.xy * c.zw
                - b.xz * c.yw
                - b.yw * c.xz
                + b.yz * c.xw
                + b.zw * c.xy,
        };

        (s, d, q)
    }

    pub fn decompose(&self) -> (Bivec4, Bivec4) {
        let pos_half_xyzw = Quadvec4::new(0.5);
        let neg_half_xyzw = Quadvec4::new(-0.5);

        let b_plus = 0.5 * *self + pos_half_xyzw.mul_bv(self);
        let b_minus = 0.5 * *self + neg_half_xyzw.mul_bv(self);

        (b_plus, b_minus)
    }

    pub fn exp(&self) -> Rotor4 {
        let (b_plus, b_minus) = self.decompose();

        let theta_plus = 2.0
            * (b_plus.xy.powi(2) + b_plus.xz.powi(2) + b_plus.xw.powi(2))
                .sqrt();
        let theta_minus = 2.0
            * (b_minus.xy.powi(2) + b_minus.xz.powi(2) + b_minus.xw.powi(2))
                .sqrt();

        let inv_theta_plus = if theta_plus > 0.0 {
            1.0 / theta_plus
        } else {
            0.0
        };
        let inv_theta_minus = if theta_minus > 0.0 {
            1.0 / theta_minus
        } else {
            0.0
        };

        let unit_b_plus = inv_theta_plus * b_plus;
        let unit_b_minus = inv_theta_minus * b_minus;

        Rotor4::new(
            0.5 * theta_plus.cos() + 0.5 * theta_minus.cos(),
            theta_plus.sin() * unit_b_plus + theta_minus.sin() * unit_b_minus,
            Quadvec4::new(0.5 * theta_plus.cos() - 0.5 * theta_minus.cos()),
        )
    }
}

impl Add<Bivec4> for Bivec4 {
    type Output = Bivec4;
    fn add(self, c: Bivec4) -> Bivec4 {
        let b = self;
        Bivec4 {
            xy: b.xy + c.xy,
            xz: b.xz + c.xz,
            xw: b.xw + c.xw,
            yz: b.yz + c.yz,
            yw: b.yw + c.yw,
            zw: b.zw + c.zw,
        }
    }
}

impl Mul<Bivec4> for f32 {
    type Output = Bivec4;
    fn mul(self, b: Bivec4) -> Bivec4 {
        Bivec4 {
            xy: self * b.xy,
            xz: self * b.xz,
            xw: self * b.xw,
            yz: self * b.yz,
            yw: self * b.yw,
            zw: self * b.zw,
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn decomp_test() {
        let b = Bivec4::new(1.0, 2.0, 3.0, 4.0, 5.0, 6.0);
        let r = b.exp();
        println!("{:?}", r);
        println!("{:?}", r.mag());
    }
}
