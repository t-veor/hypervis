use super::{Bivec4, Quadvec4, Vec4};
use std::ops::Add;
#[derive(Debug, Clone, Copy)]
pub struct Trivec4 {
    pub xyz: f32,
    pub xyw: f32,
    pub xzw: f32,
    pub yzw: f32,
}

impl Trivec4 {
    pub fn zero() -> Self {
        Self {
            xyz: 0.0,
            xyw: 0.0,
            xzw: 0.0,
            yzw: 0.0,
        }
    }

    pub fn right_contract_bv(&self, b: &Bivec4) -> Vec4 {
        let t = self;
        Vec4 {
            x: -b.yw * t.xyw - b.yz * t.xyz - b.zw * t.xzw,
            y: b.xw * t.xyw + b.xz * t.xyz - b.zw * t.yzw,
            z: b.xw * t.xzw - b.xy * t.xyz + b.yw * t.yzw,
            w: -b.xy * t.xyw - b.xz * t.xzw - b.yz * t.yzw,
        }
    }

    pub fn mul_qv(&self, q: &Quadvec4) -> Vec4 {
        let t = self;
        let xyzw = q.xyzw;
        Vec4 {
            x: xyzw * t.yzw,
            y: -xyzw * t.xzw,
            z: xyzw * t.xyw,
            w: -xyzw * t.xyz,
        }
    }
}

impl Add<Trivec4> for Trivec4 {
    type Output = Trivec4;
    fn add(self, t: Trivec4) -> Trivec4 {
        Trivec4 {
            xyz: self.xyz + t.xyz,
            xyw: self.xyw + t.xyw,
            xzw: self.xzw + t.xzw,
            yzw: self.yzw + t.yzw,
        }
    }
}
