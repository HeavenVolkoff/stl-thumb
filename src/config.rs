use image::ImageFormat;

pub struct Config {
    pub model_filename: String,
    pub img_filename: String,
    pub format: ImageFormat,
    pub width: u16,
    pub height: u16,
    pub verbosity: usize,
    pub background: (f32, f32, f32, f32),
    pub recalc_normals: bool,
    pub cam_fov_deg: f32,
    pub cam_position: (f32, f32, f32),
    /// Number of samples for anti-aliasing
    pub sample_count: u32,
}

impl Default for Config {
    #[inline]
    fn default() -> Self {
        Self {
            model_filename: String::new(),
            img_filename: String::new(),
            format: ImageFormat::Png,
            width: 1024,
            height: 1024,
            verbosity: 0,
            background: (0.0, 0.0, 0.0, 0.0),
            recalc_normals: false,
            cam_fov_deg: 45.0,
            cam_position: (2.0, -4.0, 2.0),
            sample_count: 4, // MSAA 4x
        }
    }
}
