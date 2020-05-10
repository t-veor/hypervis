use cgmath::{
    Deg, EuclideanSpace, Matrix4, One, Point3, Vector3, Vector4, Zero,
};

use super::SHADOW_SIZE;

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct Light {
    proj: Matrix4<f32>,
    position: Vector4<f32>,
    color: Vector4<f32>,
}

impl Light {
    pub fn new(position: Point3<f32>, fovy: f32, color: Vector4<f32>) -> Self {
        #[rustfmt::skip]
        let opengl_to_wgpu_matrix = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, -1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,
        );

        let aspect = SHADOW_SIZE.width as f32 / SHADOW_SIZE.height as f32;

        Self {
            proj: opengl_to_wgpu_matrix
                * cgmath::perspective(Deg(fovy), aspect, 1.0, 20.0)
                * Matrix4::look_at(
                    position,
                    Point3::origin(),
                    Vector3::unit_y(),
                ),
            position: Vector4::new(position.x, position.y, position.z, 1.0),
            color,
        }
    }
}

impl Default for Light {
    fn default() -> Self {
        Self {
            proj: Matrix4::one(),
            position: Vector4::zero(),
            color: Vector4::zero(),
        }
    }
}
