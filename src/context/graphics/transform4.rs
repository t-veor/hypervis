use cgmath::{
    prelude::{One, Zero},
    Matrix4, Vector4,
};

// Vector5s and Matrix5s are kinda annoying. We're just going to store
// transforms as a displacement and a matrix.
// Who needs homogeneous coordinates anyway?
#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Transform4 {
    pub displacement: Vector4<f32>,
    pub transform: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for Transform4 {}
unsafe impl bytemuck::Zeroable for Transform4 {}

impl Default for Transform4 {
    fn default() -> Self {
        Self {
            displacement: Vector4::zero(),
            transform: Matrix4::one(),
        }
    }
}
