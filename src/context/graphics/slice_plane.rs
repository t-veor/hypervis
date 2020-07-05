use cgmath::{
    prelude::{One, Zero},
    Matrix4, Vector4,
};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct SlicePlane {
    pub normal: Vector4<f32>,
    pub base_point: Vector4<f32>,
    pub proj_matrix: Matrix4<f32>,
}

unsafe impl bytemuck::Pod for SlicePlane {}
unsafe impl bytemuck::Zeroable for SlicePlane {}

impl Default for SlicePlane {
    fn default() -> Self {
        Self {
            normal: Vector4::unit_w(),
            base_point: Vector4::zero(),
            proj_matrix: Matrix4::one(),
        }
    }
}
