use core::slice::{from_raw_parts, from_raw_parts_mut};

use glam::Vec3;
use tracing::error;

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
    // Check that the buffer pointer is valid

    use std::path::Path;

    use crate::{render, RenderOptions};
    if output_buf.is_null() {
        error!("Image buffer pointer is null");
        return false;
    }

    let buf_size = width as usize * height as usize * 4;
    let buf = unsafe { from_raw_parts_mut(output_buf, buf_size) };

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
    let cam_position = unsafe { from_raw_parts(cam_position, 3) };

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
