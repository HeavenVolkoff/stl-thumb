use bytemuck::{Pod, Zeroable};
use glam::Mat4;
use wgpu::include_wgsl;

pub const SHADER: wgpu::ShaderModuleDescriptor<'_> = include_wgsl!("shaders/model.wgsl");

// Define the uniform data structure
#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct VertUniformBlock {
    pub(crate) perspective: Mat4,
    pub(crate) modelview: Mat4,
}

#[repr(C, align(16))]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FragUniformBlock {
    pub(crate) light_direction: [f32; 3],
    _padding1: [u8; 4],
    pub(crate) ambient_color: [f32; 3],
    _padding2: [u8; 4],
    pub(crate) diffuse_color: [f32; 3],
    _padding3: [u8; 4],
    pub(crate) specular_color: [f32; 3],
    _padding4: [u8; 4],
}

impl FragUniformBlock {
    #[inline]
    pub const fn new(
        light_direction: [f32; 3],
        ambient_color: [f32; 3],
        diffuse_color: [f32; 3],
        specular_color: [f32; 3],
    ) -> Self {
        Self {
            light_direction,
            ambient_color,
            diffuse_color,
            specular_color,
            _padding1: [0; 4],
            _padding2: [0; 4],
            _padding3: [0; 4],
            _padding4: [0; 4],
        }
    }
}
