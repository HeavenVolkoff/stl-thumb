use image::ImageFormat;

/// Matches a file extension to an `ImageFormat`.
///
/// # Arguments
///
/// * `ext` - A string slice that holds the file extension.
///
/// # Returns
///
/// * `ImageFormat` - The corresponding image format.
pub fn match_format(ext: &str) -> ImageFormat {
    match ext.to_lowercase().as_str() {
        "jpeg" | "jpg" => ImageFormat::Jpeg,
        "gif" => ImageFormat::Gif,
        "ico" => ImageFormat::Ico,
        "bmp" => ImageFormat::Bmp,
        _ => ImageFormat::Png,
    }
}

/// Converts an HTML color code to an RGBA tuple.
///
/// # Arguments
///
/// * `color` - A string slice that holds the HTML color code.
///
/// # Returns
///
/// * `(f32, f32, f32, f32)` - A tuple containing the RGBA values.
///
/// # Panics
///
/// * Panics if the color string is not a valid HTML color code.
pub fn html_to_rgba(color: &str) -> (f32, f32, f32, f32) {
    assert!(
        color.len() == 8,
        "Invalid color length, expected 8 characters"
    );

    let red =
        f32::from(u8::from_str_radix(&color[0..2], 16).expect("Invalid red component")) / 255.0;
    let green =
        f32::from(u8::from_str_radix(&color[2..4], 16).expect("Invalid green component")) / 255.0;
    let blue =
        f32::from(u8::from_str_radix(&color[4..6], 16).expect("Invalid blue component")) / 255.0;
    let alpha =
        f32::from(u8::from_str_radix(&color[6..8], 16).expect("Invalid alpha component")) / 255.0;

    (red, green, blue, alpha)
}
