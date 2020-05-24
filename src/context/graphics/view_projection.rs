use crate::context::Ctx;
use cgmath::{Deg, Matrix4, One, Point3, SquareMatrix, Vector3, Vector4};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ViewProjection {
    view_proj: Matrix4<f32>,
}

impl ViewProjection {
    pub fn new(
        ctx: &Ctx,
        fovy: f32,
        look_from: Point3<f32>,
        look_at: Point3<f32>,
    ) -> Self {
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
                * cgmath::perspective(Deg(fovy), aspect, 1.0, 100.0)
                * Matrix4::look_at(look_from, look_at, Vector3::unit_y()),
        }
    }

    pub fn world_to_screen(&self, world: Vector4<f32>) -> Vector4<f32> {
        self.view_proj * world
    }

    pub fn screen_to_world(&self, screen: Vector4<f32>) -> Vector4<f32> {
        self.view_proj.invert().unwrap() * screen
    }
}

impl Default for ViewProjection {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::one(),
        }
    }
}
