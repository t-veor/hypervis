use cgmath::Vector4;

#[derive(Clone)]
pub enum Collider {
    HalfPlane { normal: Vector4<f32> },
    Tesseract { half_width: f32 },
}
