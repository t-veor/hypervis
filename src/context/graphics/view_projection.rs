use crate::context::Ctx;
use cgmath::{prelude::One, Deg, Matrix4, Point3, Vector3};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ViewProjection {
    view_proj: Matrix4<f32>,
}

impl ViewProjection {
    pub fn new(ctx: &Ctx) -> Self {
        #[rustfmt::skip]
        let opengl_to_wgpu_matrix = Matrix4::new(
            1.0, 0.0, 0.0, 0.0,
            0.0, -1.0, 0.0, 0.0,
            0.0, 0.0, 0.5, 0.0,
            0.0, 0.0, 0.5, 1.0,
        );

        let aspect = ctx.graphics_ctx.sc_desc.width as f32
            / ctx.graphics_ctx.sc_desc.height as f32;

        Self {
            view_proj: opengl_to_wgpu_matrix
                * cgmath::perspective(Deg(90.0), aspect, 0.1, 1000.0)
                * Matrix4::look_at(
                    Point3::new(1.0, 1.0, -2.0) * 2.0,
                    Point3::new(0.0, 1.0, 0.0),
                    Vector3::unit_y(),
                ),
        }
    }
}

impl Default for ViewProjection {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::one(),
        }
    }
}
