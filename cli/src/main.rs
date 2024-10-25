mod utils;

extern crate clap;
extern crate md5;
extern crate stl_thumb;
extern crate tokio;
extern crate tracing;
extern crate tracing_subscriber;

use std::path::Path;

use clap::{Arg, ArgAction, Command};
use stl_thumb::{render, render_to_file, Config};

use crate::utils::{html_to_rgba, match_format};

fn args() -> Result<(Config, bool), Box<dyn std::error::Error>> {
    let matches = Command::new(env!("CARGO_PKG_NAME"))
        .version(env!("CARGO_PKG_VERSION"))
        .author(env!("CARGO_PKG_AUTHORS"))
        .about("Generate thumbnails for STL files")
        .arg(
            Arg::new("MODEL_FILE")
                .help("STL file. Use - to read from stdin instead of a file.")
                .required(true)
                .index(1),
        )
        .arg(
            Arg::new("IMG_FILE")
                .help("Thumbnail image file. Use - to write to stdout instead of a file.")
                .required(true)
                .index(2),
        )
        .arg(
            Arg::new("format")
                .help("The format of the image file. Supported formats: PNG, JPEG, GIF, ICO, BMP")
                .short('f')
                .long("format")
                .action(ArgAction::Set)
                .value_parser(["png", "jpeg", "gif", "ico", "bmp"]),
        )
        .arg(
            Arg::new("size")
                .help("Size of thumbnail (square) or <width>x<height>")
                .short('s')
                .long("size")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("verbosity")
                .help("Increase message verbosity")
                .short('v')
                .long("verbosity")
                .action(ArgAction::Count),
        )
        .arg(
            Arg::new("background")
                .help("The background color with transparency (rgba). Default is ffffff00.")
                .short('b')
                .long("background")
                .action(ArgAction::Set)
        )
        .arg(
            Arg::new("recalc_normals")
                .help("Force recalculation of face normals. Use when dealing with malformed STL files.")
                .long("recalc-normals")
                .action(ArgAction::SetTrue),
        )
        .arg(
            Arg::new("cam_fov_deg")
                .help("Camera field of view in degrees")
                .long("cam-fov-deg")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(f32))
        )
        .arg(
            Arg::new("cam_position")
                .help("Camera position as a comma-separated list of three floats (x,y,z)")
                .long("cam-position")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(String))
        )
        .arg(
            Arg::new("sample_count")
                .help("Number of samples for rendering")
                .long("sample-count")
                .action(ArgAction::Set)
                .value_parser(clap::value_parser!(u32))
        )
        .arg(
            Arg::new("md5")
                .help("Calculate MD5 hash of the rendered model")
                .long("md5")
                .action(ArgAction::SetTrue),
        )
        .get_matches();

    let mut c = Config {
        model_filename: matches
            .get_one::<String>("MODEL_FILE")
            .ok_or("MODEL_FILE not provided")?
            .to_string(),
        img_filename: matches
            .get_one::<String>("IMG_FILE")
            .ok_or("IMG_FILE not provided")?
            .to_string(),
        verbosity: matches.get_count("verbosity").into(),
        recalc_normals: matches.get_flag("recalc_normals"),
        ..Default::default()
    };

    if let Some(size) = matches.get_one::<String>("size") {
        if let Ok(size_num) = size.parse::<u16>() {
            c.width = size_num;
            c.height = size_num;
        } else if let Some((width, height)) = size.split_once('x') {
            c.width = width
                .trim()
                .parse::<u16>()
                .map_err(|_| "Invalid width in size")?;
            c.height = height
                .trim()
                .parse::<u16>()
                .map_err(|_| "Invalid height in size")?;
        } else {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                "Invalid size format. Use a number or <width>x<height>",
            )));
        }
    }

    if let Some(format) = matches.get_one::<String>("format") {
        c.format = match_format(format);
    }

    if let Some(background) = matches.get_one::<String>("background") {
        c.background = html_to_rgba(background);
    }

    if let Some(cam_fov_deg) = matches.get_one::<f32>("cam_fov_deg") {
        c.cam_fov_deg = *cam_fov_deg;
    }

    if let Some(sample_count) = matches.get_one::<u32>("sample_count") {
        c.sample_count = *sample_count;
    }

    if let Some(cam_position) = matches.get_one::<String>("cam_position") {
        let cam_position_vec = cam_position
            .split(',')
            .map(|s| {
                s.trim()
                    .parse::<f32>()
                    .map_err(|_| "Invalid float in cam_position")
            })
            .collect::<Result<Vec<f32>, _>>()?;

        if cam_position_vec.len() != 3 {
            return Err(Box::new(std::io::Error::new(
                std::io::ErrorKind::InvalidInput,
                format!(
                    "cam_position must have exactly three elements, but got {}",
                    cam_position_vec.len()
                ),
            )));
        }

        c.cam_position = (
            cam_position_vec[0],
            cam_position_vec[1],
            cam_position_vec[2],
        );
    }

    if matches.get_flag("md5") && c.img_filename != "-" {
        return Err(Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "IMG_FILE must be '-' when using --md5",
        )));
    };

    Ok((c, matches.get_flag("md5")))
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (config, md5) = args()?;

    tracing_subscriber::fmt()
        .with_max_level(match config.verbosity {
            0 => tracing::Level::ERROR,
            1 => tracing::Level::WARN,
            2 => tracing::Level::INFO,
            3 => tracing::Level::DEBUG,
            _ => tracing::Level::TRACE,
        })
        .with_writer(std::io::stderr)
        .init();

    if md5 {
        let digest =
            md5::compute(&render(Path::new(&config.model_filename), &(&config).into()).await?);
        println!("MD5: {:x}", digest);
    } else {
        render_to_file(
            Path::new(&config.model_filename),
            Path::new(&config.img_filename),
            config.format,
            &(&config).into(),
        )
        .await?;
    }

    Ok(())
}

// Notes
// =====
//
// Linux Thumbnails
// ----------------
// https://tecnocode.co.uk/2013/10/21/writing-a-gnome-thumbnailer/
// https://wiki.archlinux.org/index.php/XDG_MIME_Applications#Shared_MIME_database
// https://developer.gnome.org/integration-guide/stable/thumbnailer.html.en (outdated)
//
// Window Thumbnails
// -----------------
// https://code.msdn.microsoft.com/windowsapps/CppShellExtThumbnailHandler-32399b35
// https://github.com/Arlorean/Voxels
//
// Helpful Examples
// ----------------
// https://github.com/bwasty/gltf-viewer
//
// OpenGL
// ------
// https://glium-doc.github.io/#/
// http://www.opengl-tutorial.org/beginners-tutorials/tutorial-3-matrices/
