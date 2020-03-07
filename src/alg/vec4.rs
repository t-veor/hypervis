use super::{Bivec4, Trivec4};
use mint::Vector4;
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy)]
pub struct Vec4 {
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub w: f32,
}

impl Vec4 {
    pub fn zero() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            z: 0.0,
            w: 0.0,
        }
    }

    pub fn left_contract_bv(&self, b: &Bivec4) -> Vec4 {
        let v = self;

        Vec4 {
            x: -v.y * b.xy - v.z * b.xz - v.w * b.xw,
            y: v.x * b.xy - v.z * b.yz - v.w * b.yw,
            z: v.x * b.xz + v.y * b.yz - v.w * b.zw,
            w: v.x * b.xw + v.y * b.yw + v.z * b.zw,
        }
    }

    pub fn wedge_bv(&self, b: &Bivec4) -> Trivec4 {
        let v = self;

        Trivec4 {
            xyz: v.x * b.yz - v.y * b.xz + v.z * b.xy,
            xyw: v.x * b.yw - v.y * b.xw + v.w * b.xy,
            xzw: v.x * b.zw - v.z * b.xw + v.w * b.xz,
            yzw: v.x * b.zw - v.z * b.yw + v.w * b.yz,
        }
    }

    pub fn wedge_v(&self, v: &Vec4) -> Bivec4 {
        let u = self;
        Bivec4 {
            xy: u.x * v.y - u.y * v.x,
            xz: u.x * v.z - u.z * v.x,
            xw: -u.w * v.x + u.x * v.w,
            yz: u.y * v.z - u.z * v.y,
            yw: -u.w * v.y + u.y * v.w,
            zw: -u.w * v.z + u.z * v.w,
        }
    }

    pub fn mul_bv(&self, b: &Bivec4) -> (Vec4, Trivec4) {
        (self.left_contract_bv(b), self.wedge_bv(b))
    }
}

impl Mul<Vec4> for f32 {
    type Output = Vec4;
    fn mul(self, v: Vec4) -> Vec4 {
        Vec4 {
            x: self * v.x,
            y: self * v.y,
            z: self * v.z,
            w: self * v.w,
        }
    }
}

impl Add<Vec4> for Vec4 {
    type Output = Vec4;
    fn add(self, v: Vec4) -> Vec4 {
        let u = self;
        Vec4 {
            x: u.x + v.x,
            y: u.y + v.y,
            z: u.z + v.z,
            w: u.w + v.w,
        }
    }
}

impl Into<Vector4<f32>> for Vec4 {
    fn into(self) -> Vector4<f32> {
        Vector4 {
            x: self.x,
            y: self.y,
            z: self.z,
            w: self.w,
        }
    }
}

impl From<Vector4<f32>> for Vec4 {
    fn from(v: Vector4<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }
}

impl Into<cgmath::Vector4<f32>> for Vec4 {
    fn into(self) -> cgmath::Vector4<f32> {
        cgmath::Vector4::new(self.x, self.y, self.z, self.w)
    }
}

impl From<cgmath::Vector4<f32>> for Vec4 {
    fn from(v: cgmath::Vector4<f32>) -> Self {
        Self {
            x: v.x,
            y: v.y,
            z: v.z,
            w: v.w,
        }
    }
}
