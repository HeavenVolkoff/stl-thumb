use std::io;

use thiserror::Error;

#[non_exhaustive]
#[derive(Error, Debug)]
pub enum MeshError {
    #[error("IO error")]
    Io(#[from] io::Error),
    #[error("STL error")]
    Obj(#[from] tobj::LoadError),
    #[error("3MF error")]
    Threemf(#[from] threemf::Error),
    #[error("Unsupported format")]
    UnsupportedFormat,
    #[error("Invalid STL: {0}")]
    InvalidStl(String),
    #[error("Invalid STL: {0}")]
    InvalidObj(String),
    #[error("Invalid 3MF: {0}")]
    InvalidThreemf(String),
    #[error("Empty mesh")]
    EmptyMesh,
    #[error("No mesh data found in 3MF file")]
    NoMeshData,
}

#[non_exhaustive]
#[derive(Error, Debug)]
/// Errors that can occur during rendering operations.
pub enum RenderError {
    /// Error related to WGPU requests.
    #[error("WGPU Error {0}")]
    Wgpu(#[from] wgpu::RequestDeviceError),
    /// General render operation failure.
    #[error("Render operation failed: {0}")]
    RenderError(String),
}
