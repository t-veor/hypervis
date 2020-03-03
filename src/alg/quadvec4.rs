use super::{Bivec4, Trivec4, Vec4};
use std::ops::{Add, Mul};

#[derive(Debug, Copy, Clone)]
pub struct Quadvec4 {
    pub xyzw: f32,
}

impl Quadvec4 {
    pub fn new(xyzw: f32) -> Self {
        Self { xyzw }
    }

    pub fn zero() -> Self {
        Self { xyzw: 0.0 }
    }

    pub fn mul_v(&self, v: &Vec4) -> Trivec4 {
        let q = self;
        Trivec4 {
            xyz: q.xyzw * v.w,
            xyw: -q.xyzw * v.z,
            xzw: q.xyzw * v.y,
            yzw: -q.xyzw * v.x,
        }
    }

    pub fn mul_bv(&self, b: &Bivec4) -> Bivec4 {
        let xyzw = self.xyzw;
        Bivec4 {
            xy: -b.zw * xyzw,
            xz: b.yw * xyzw,
            xw: -b.yz * xyzw,
            yz: -b.xw * xyzw,
            yw: b.xz * xyzw,
            zw: -b.xy * xyzw,
        }
    }
}

impl Add<Quadvec4> for Quadvec4 {
    type Output = Quadvec4;
    fn add(self, q: Quadvec4) -> Quadvec4 {
        Quadvec4 {
            xyzw: self.xyzw + q.xyzw,
        }
    }
}

impl Mul<Quadvec4> for f32 {
    type Output = Quadvec4;
    fn mul(self, q: Quadvec4) -> Quadvec4 {
        Quadvec4 {
            xyzw: self * q.xyzw,
        }
    }
}
