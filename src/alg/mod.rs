mod bivec4;
mod quadvec4;
mod rotor4;
mod trivec4;
mod vec4;

pub use bivec4::Bivec4;
use quadvec4::Quadvec4;
pub use rotor4::Rotor4;
pub use trivec4::Trivec4;
pub use vec4::Vec4;

use cgmath::Vector4;

// Gets a vector that's perpendicular to all three vectors given.
pub fn triple_cross_product(
    u: Vector4<f32>,
    v: Vector4<f32>,
    w: Vector4<f32>,
) -> Vector4<f32> {
    let u: Vec4 = u.into();
    u.wedge_v(&v.into())
        .wedge_v(&w.into())
        .mul_qv(&Quadvec4::one())
        .into()
}
