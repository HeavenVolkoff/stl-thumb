#![cfg_attr(not(feature = "capi"), deny(unsafe_code))]

#[cfg(feature = "capi")]
mod capi;
mod config;
mod error;
mod mesh;
mod render;
mod shader;

use std::{error::Error, path::Path};

#[cfg(feature = "image")]
use image::{ImageBuffer, ImageEncoder, ImageFormat, Rgba};
use mesh::Mesh;

#[cfg(feature = "capi")]
pub use crate::capi::*;
use crate::render::ThumbRenderer;
pub use crate::{
    config::Config,
    error::{MeshError, RenderError},
    render::RenderOptions,
};

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
    Ok(ThumbRenderer::new(opts.sample_count).await?.render(
        &Mesh::load(
            model_filename.to_str().ok_or("Invalid path")?,
            opts.recalc_normals,
        )?,
        opts,
    )?)
}

/// Renders a 3D model to an image.
///
/// # Errors
///
/// This function will return an error if the model file cannot be loaded,
/// or if the rendering process fails.
#[cfg(feature = "image")]
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
#[cfg(feature = "image")]
pub async fn render_to_file(
    model_filename: &Path,
    img_filename: &Path,
    format: ImageFormat,
    opts: &RenderOptions,
) -> Result<(), Box<dyn Error>> {
    use std::{ffi, fs, io};

    let img = render_to_image(model_filename, opts).await?;

    // Choose output
    // Write to stdout if user did not specify a file
    let mut output: Box<dyn io::Write> = match &img_filename {
        os if os == ffi::OsStr::new("-") => Box::new(io::stdout()),
        out => Box::new(fs::File::create(out)?),
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

#[cfg(feature = "image")]
#[cfg(test)]
mod tests {
    #![allow(clippy::borrow_interior_mutable_const)]

    use std::{cell::LazyCell, fs, io::ErrorKind};

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
            Path::new("test/data/cube.stl"),
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
            Path::new("test/data/cube.obj"),
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
            Path::new("test/data/cube.3mf"),
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
