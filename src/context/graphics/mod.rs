pub mod context;
pub mod light;
pub mod shadow_pipeline;
pub mod slice_pipeline;
pub mod slice_plane;
pub mod transform4;
pub mod triangle_list_pipeline;
pub mod vertex3;
pub mod vertex4;
pub mod view_projection;

pub use context::*;
pub use light::*;
pub use shadow_pipeline::*;
pub use slice_pipeline::*;
pub use slice_plane::*;
pub use transform4::*;
pub use triangle_list_pipeline::*;
pub use vertex3::*;
pub use vertex4::*;
pub use view_projection::*;

pub const DEPTH_FORMAT: wgpu::TextureFormat = wgpu::TextureFormat::Depth32Float;

pub const SHADOW_FORMAT: wgpu::TextureFormat =
    wgpu::TextureFormat::Depth32Float;
pub const SHADOW_SIZE: wgpu::Extent3d = wgpu::Extent3d {
    width: 1024,
    height: 1024,
    depth: 1,
};
