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

#[repr(C)]
#[derive(Copy, Clone, Debug, Pod, Zeroable)]
pub struct FragUniformBlock {
    pub(crate) light_direction: [f32; 3],
    pub(crate) _padding1: f32, // Padding to align to 16 bytes
    pub(crate) ambient_color: [f32; 3],
    pub(crate) _padding2: f32, // Padding to align to 16 bytes
    pub(crate) diffuse_color: [f32; 3],
    pub(crate) _padding3: f32, // Padding to align to 16 bytes
    pub(crate) specular_color: [f32; 3],
    pub(crate) _padding4: f32, // Padding to align to 16 bytes
}
