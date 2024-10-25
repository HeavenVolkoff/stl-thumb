mod config;
mod error;
mod mesh;
mod render;
mod shader;

use std::error::Error;
use std::io;
use std::path::Path;

pub use config::Config;
pub use error::{MeshError, RenderError};
use glam::Vec3;
use image::{ImageBuffer, ImageEncoder, ImageFormat, Rgba};
use mesh::Mesh;

use crate::render::ThumbRenderer;

pub struct RenderOptions {
    pub width: u16,
    pub height: u16,
    pub cam_fov_deg: f32,
    pub cam_position: Vec3,
    pub sample_count: u32,
    pub recalc_normals: bool,
}

impl From<&Config> for RenderOptions {
    fn from(config: &Config) -> Self {
        Self {
            width: config.width,
            height: config.height,
            cam_fov_deg: config.cam_fov_deg,
            cam_position: config.cam_position.into(),
            sample_count: config.sample_count,
            recalc_normals: config.recalc_normals,
        }
    }
}

/// Renders a 3D model to a buffer.
///
/// # Errors
///
/// This function will return an error if the model file cannot be loaded,
/// or if the rendering process fails.
pub async fn render(
    model_filename: &Path,
    opts: &RenderOptions,
) -> Result<Vec<u8>, Box<dyn Error>> {
    Ok(ThumbRenderer::new(opts.sample_count)
        .await?
        .render(
            &Mesh::load(
                model_filename.to_str().ok_or("Invalid path")?,
                opts.recalc_normals,
            )?,
            opts,
        )
        .await?)
}

/// Renders a 3D model to an image.
///
/// # Errors
///
/// This function will return an error if the model file cannot be loaded,
/// or if the rendering process fails.
pub async fn render_to_image(
    filename: &Path,
    opts: &RenderOptions,
) -> Result<image::DynamicImage, Box<dyn Error>> {
    let buffer = render(filename, opts).await?;

    // Create image from the raw pixel data
    Ok(image::DynamicImage::ImageRgba8(
        ImageBuffer::<Rgba<u8>, _>::from_raw(u32::from(opts.width), u32::from(opts.height), buffer)
            .ok_or("Failed to create image buffer")?,
    ))
}

/// Renders a 3D model to an image file.
///
/// # Errors
///
/// This function will return an error if the model file cannot be loaded,
/// if the rendering process fails, or if the image cannot be written to the file.
pub async fn render_to_file(
    model_filename: &Path,
    img_filename: &Path,
    format: ImageFormat,
    opts: &RenderOptions,
) -> Result<(), Box<dyn Error>> {
    let img = render_to_image(model_filename, opts).await?;

    // Choose output
    // Write to stdout if user did not specify a file
    let mut output: Box<dyn io::Write> = match &img_filename {
        os if os == std::ffi::OsStr::new("-") => Box::new(io::stdout()),
        out => Box::new(std::fs::File::create(out)?),
    };

    // write_to() requires a seekable writer for performance reasons.
    // So we create an in-memory buffer and then dump that to the output.
    // I wonder if it would be better to use std::io::BufWriter for writing files instead.
    let mut buff: Vec<u8> = Vec::new();
    let mut cursor = io::Cursor::new(&mut buff);

    // Encode image with specified format
    // If encoding a PNG image, use fastest compression method
    // Not sure if this is really necessary. Fast is the default anyways.
    match format {
        ImageFormat::Png => {
            let encoder = image::codecs::png::PngEncoder::new_with_quality(
                &mut cursor,
                image::codecs::png::CompressionType::Fast,
                //image::codecs::png::CompressionType::Default,
                image::codecs::png::FilterType::Adaptive,
            );
            encoder.write_image(
                img.as_bytes(),
                u32::from(opts.width),
                u32::from(opts.height),
                img.color().into(),
            )?;
        }
        format => img.write_to(&mut cursor, format)?,
    }

    output.write_all(&buff)?;
    output.flush()?;

    Ok(())
}

/// Allows utilizing `stl-thumb` from C-like languages
///
/// This function renders an image of the file `filename` and stores it into the buffer `output_buf`.
///
/// You must provide a memory buffer large enough to store the image. Images are written in 8-bit RGBA format,
/// so the buffer must be at least `width`*`height`*4 bytes in size. `filename` is a pointer to a C string with
/// the file path.
///
/// Returns `true` if successful and `false` if unsuccessful.
///
/// # Example in C
/// ```c
/// const char* filename = "3DBenchy.stl";
/// int width = 256;
/// int height = 256;
/// float cam_fov_deg = 45.0;
/// float cam_position[3] = {0.0, 0.0, 5.0};
///
/// int img_size = width * height * 4;
/// output_buf = (uchar *) malloc(img_size);
///
/// render_to_buffer(filename, width, height, cam_fov_deg, cam_position, output_buf);
/// ```
///
/// # Safety
///
/// * `output_buf` _must_ point to a valid initialized buffer, at least `width * height * 4` bytes long.
/// * `filename` must point to a valid null-terminated string.
/// * `cam_position` must point to an array of 3 floats.
#[no_mangle]
pub unsafe extern "C" fn render_to_buffer(
    filename: *const libc::c_char,
    width: u16,
    height: u16,
    cam_fov_deg: f32,
    cam_position: *const f32,
    sample_count: u32,
    recalc_normals: bool,
    output_buf: *mut u8,
) -> bool {
    use tracing::error;

    // Check that the buffer pointer is valid
    if output_buf.is_null() {
        error!("Image buffer pointer is null");
        return false;
    }
    let buf_size = (width * height * 4) as usize;
    let buf = unsafe { core::slice::from_raw_parts_mut(output_buf, buf_size) };

    // Check validity of provided file path string
    if filename.is_null() {
        error!("model file path pointer is null");
        return false;
    }
    let filename = unsafe { std::ffi::CStr::from_ptr(filename) };
    let Ok(filename) = filename.to_str() else {
        error!("Invalid model file path {:?}", filename);
        return false;
    };

    // Check that the camera position pointer is valid
    if cam_position.is_null() {
        error!("Camera position pointer is null");
        return false;
    }
    let cam_position = unsafe { core::slice::from_raw_parts(cam_position, 3) };

    // Create a runtime to block on the async function
    let runtime = match tokio::runtime::Runtime::new() {
        Ok(rt) => rt,
        Err(e) => {
            error!("Failed to create Tokio runtime: {:?}", e);
            return false;
        }
    };

    // Render the image
    let render_opts = RenderOptions {
        width,
        height,
        cam_fov_deg,
        cam_position: Vec3::new(cam_position[0], cam_position[1], cam_position[2]),
        sample_count,
        recalc_normals,
    };

    let buffer = match runtime.block_on(render(Path::new(filename), &render_opts)) {
        Ok(buf) => buf,
        Err(e) => {
            error!("Rendering error: {:?}", e);
            return false;
        }
    };

    // Copy the rendered buffer to the output buffer
    buf.copy_from_slice(&buffer);

    true
}
/// Parameters for rendering a 3D model.
pub struct RenderParams {
    pub width: usize,
    pub height: usize,
    pub cam_fov: f32,
    pub cam_pos: Vec3,
    pub sample_count: u32,
    pub recalc_normals: bool,
}

#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)]

    use std::cell::LazyCell;
    use std::fs;
    use std::io::ErrorKind;

    use config::Config;

    use super::*;

    #[allow(clippy::declare_interior_mutable_const)]
    const CONFIG: LazyCell<Config> = LazyCell::new(Config::default);

    #[tokio::test]
    async fn cube_stl() {
        let img_filename = Path::new("cube-stl.png");
        match fs::remove_file(img_filename) {
            Err(ref error) if error.kind() == ErrorKind::NotFound => (),
            r => r.expect("Couldn't clean files before testing"),
        }

        render_to_file(
            Path::new("test_data/cube.stl"),
            img_filename,
            ImageFormat::Png,
            &(&*CONFIG).into(),
        )
        .await
        .expect("Error in render function");

        let size = fs::metadata(img_filename).expect("No file created").len();

        assert_ne!(0, size);
    }

    #[tokio::test]
    async fn cube_obj() {
        let img_filename = Path::new("cube-obj.png");
        match fs::remove_file(img_filename) {
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            r => r.expect("Couldn't clean files before testing"),
        }

        render_to_file(
            Path::new("test_data/cube.obj"),
            img_filename,
            ImageFormat::Png,
            &(&*CONFIG).into(),
        )
        .await
        .expect("Error in render function");

        let size = fs::metadata(img_filename).expect("No file created").len();

        assert_ne!(0, size);
    }

    #[tokio::test]
    async fn cube_3mf() {
        let img_filename = Path::new("cube-3mf.png");
        match fs::remove_file(img_filename) {
            Err(error) if error.kind() == ErrorKind::NotFound => (),
            r => r.expect("Couldn't clean files before testing"),
        }

        render_to_file(
            Path::new("test_data/cube.3mf"),
            img_filename,
            ImageFormat::Png,
            &(&*CONFIG).into(),
        )
        .await
        .expect("Error in render function");

        let size = fs::metadata(img_filename).expect("No file created").len();

        assert_ne!(0, size);
    }
}
