use crate::alg::triple_cross_product;
use crate::context::Ctx;
use cgmath::{Deg, InnerSpace, Matrix4, One, Point3, Vector3, Vector4, Zero};

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ViewProjection3 {
    view_proj: Matrix4<f32>,
}

impl ViewProjection3 {
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
                * cgmath::perspective(Deg(fovy), aspect, 0.1, 1000.0)
                * Matrix4::look_at(look_from, look_at, Vector3::unit_y()),
        }
    }
}

impl Default for ViewProjection3 {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::one(),
        }
    }
}

#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct ViewProjection4 {
    view_proj: Matrix4<f32>,
    translate: Vector4<f32>,
}

impl ViewProjection4 {
    pub fn new(
        fovy: f32,
        look_from: Vector4<f32>,
        look_at: Vector4<f32>,
        back: Vector4<f32>,
    ) -> Self {
        let up = Vector4::unit_y();

        let look_at_matrix = {
            let col3 = (look_at - look_from).normalize();
            let col2 = triple_cross_product(up, back, col3).normalize();
            let col1 = triple_cross_product(back, col2, col3).normalize();
            let col0 = triple_cross_product(col1, col2, col3).normalize();
            Matrix4::from_cols(col0, col1, col2, col3)
        };

        let perspective_factor = 1.0 / (fovy.to_radians() / 2.0).tan();
        let perspective_matrix = Matrix4::from_cols(
            Vector4::unit_x() * perspective_factor,
            Vector4::unit_y() * perspective_factor,
            Vector4::unit_z() * perspective_factor,
            Vector4::unit_w(),
        );

        Self {
            view_proj: perspective_matrix * look_at_matrix,
            translate: -look_from,
        }
    }
}

impl Default for ViewProjection4 {
    fn default() -> Self {
        Self {
            view_proj: Matrix4::one(),
            translate: Vector4::zero(),
        }
    }
}
