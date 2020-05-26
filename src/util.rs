use cgmath::{InnerSpace, Vector3, Vector4};
use std::cmp::Ordering;
use std::hint::unreachable_unchecked;

pub const EPSILON: f32 = 1e-6;

// Wrapper around a float that implements Ord.
#[derive(PartialOrd, PartialEq, Debug, Default, Clone, Copy)]
pub struct NotNaN(f32);

impl NotNaN {
    pub fn new(f: f32) -> Option<Self> {
        if f.is_nan() {
            None
        } else {
            Some(NotNaN(f))
        }
    }

    pub fn into_inner(self) -> f32 {
        self.0
    }
}

impl Eq for NotNaN {}

impl Ord for NotNaN {
    fn cmp(&self, other: &Self) -> Ordering {
        match self.partial_cmp(other) {
            Some(ord) => ord,
            None => unsafe { unreachable_unchecked() },
        }
    }
}

// Extension of https://box2d.org/posts/2014/02/computing-a-basis/ to 4 dimensions.
// (Refer to https://www.geometrictools.com/Documentation/OrthonormalSets.pdf)
pub fn orthonormal_basis(a: Vector4<f32>) -> [Vector4<f32>; 3] {
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
