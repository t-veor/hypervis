use cgmath::Vector4;

#[derive(Clone)]
pub enum Collider {
    HalfSpace { normal: Vector4<f32> },
    Tesseract { half_width: f32 },
}
