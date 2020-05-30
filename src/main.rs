use glob::glob;
use image::{imageops, GenericImageView};
use log::{info, warn};
use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

#[cfg(test)]
mod tests;

struct ImgBuf {
    pub buf: Vec<u32>,
    pub width: usize,
    pub height: usize,
}
#[derive(Copy, Clone, Debug)]
struct WindowSize(usize, usize);

#[derive(Debug)]
enum ImageFilepathError {
    InvalidDirectory,
    InvalidCharset,
    InvalidGlobPattern(glob::PatternError),
    NoImageFileFound,
}

#[derive(Debug)]
enum ImageBufferError {
    OpenError(image::error::ImageError),
    RgbParseError,
}

impl fmt::Display for ImageFilepathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ImageFilepathError::InvalidDirectory => write!(f, "Invalid directory path"),
            ImageFilepathError::InvalidCharset => write!(f, "Use UTF-8 for photo directory name"),
            ImageFilepathError::InvalidGlobPattern(ref e) => e.fmt(f),
            ImageFilepathError::NoImageFileFound => write!(f, "No image file found"),
        }
    }
}

impl From<glob::PatternError> for ImageFilepathError {
    fn from(e: glob::PatternError) -> ImageFilepathError {
        ImageFilepathError::InvalidGlobPattern(e)
    }
}

impl fmt::Display for ImageBufferError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ImageBufferError::OpenError(ref e) => e.fmt(f),
            ImageBufferError::RgbParseError => write!(f, "Failed to parse image file as RGB8"),
        }
    }
}

impl From<image::error::ImageError> for ImageBufferError {
    fn from(e: image::error::ImageError) -> ImageBufferError {
        ImageBufferError::OpenError(e)
    }
}

fn get_scaled_img_filepath_array<P>(
    dir: P,
    window_size: WindowSize,
) -> Result<Vec<PathBuf>, ImageFilepathError>
where
    P: AsRef<Path>,
{
    if !dir.as_ref().is_dir() {
        return Err(ImageFilepathError::InvalidDirectory);
    }

    let filepath_base = dir.as_ref().join("*.jpg");
    let pat = match filepath_base.to_str() {
        Some(p) => p,
        None => return Err(ImageFilepathError::InvalidCharset),
    };
    let glob_pat = glob(pat)?;

    let mut img_filepaths: Vec<PathBuf> = vec![];
    for entry in glob_pat {
        if let Ok(s) = entry {
            let s_file_name = match s.file_name() {
                Some(n) => n,
                None => {
                    warn!("Failed to get filename of {}", s.display());
                    continue;
                }
            };

            let mut img = match image::open(&s) {
                Ok(i) => i,
                Err(_) => {
                    warn!("Failed to read image {}", s.display());
                    continue;
                }
            };

            let resized_dir = s
                .parent()
                .unwrap_or_else(|| panic!("Failed to get parent directory of {:?}", s))
                .join("resized");
            if img.dimensions().0 > window_size.0 as u32
                || img.dimensions().1 > window_size.1 as u32
            {
                // resize big images to load fast

                // create `resized` directory
                if !Path::new(&resized_dir).exists() {
                    fs::create_dir(&resized_dir).unwrap_or_else(|_| {
                        panic!("Failed to create directory {}", resized_dir.display())
                    });
                    info!("created directory {}", resized_dir.display());
                }

                // for resize algorithm detail, see official documents at
                // https://docs.rs/image/0.23.4/image/imageops/enum.FilterType.html#examples
                img = img.resize(
                    window_size.0 as u32,
                    window_size.1 as u32,
                    imageops::CatmullRom,
                );
                let resized_filename = resized_dir.join(s_file_name);
                match img.save(&resized_filename) {
                    Ok(_) => img_filepaths.push(resized_filename),
                    Err(_) => warn!("Failed to save {}", resized_filename.display()),
                };
            } else {
                img_filepaths.push(PathBuf::from(&s));
            }
        } else {
            warn!("Failed to parse path {:?}", entry);
        }
    }
    if !img_filepaths.is_empty() {
        Ok(img_filepaths)
    } else {
        Err(ImageFilepathError::NoImageFileFound)
    }
}

fn image_buffer_from_filepath<P>(filepath: P) -> Result<ImgBuf, ImageBufferError>
where
    P: AsRef<Path>,
{
    let img = image::open(&filepath)?;
    let rgb = match img.as_rgb8() {
        Some(r) => r,
        None => return Err(ImageBufferError::RgbParseError),
    };

    let (width, height) = rgb.dimensions();
    let mut buf: Vec<u32> = vec![0; (width * height) as usize];

    let threads = num_cpus::get();
    let rows_per_band = height / threads as u32 + 1;
    let bands: Vec<&mut [u32]> = buf.chunks_mut((rows_per_band * width) as usize).collect();
    bands.into_par_iter().enumerate().for_each(|(i, band)| {
        for (j, b) in band.iter_mut().enumerate() {
            let x = j as u32 % width;
            let y = i as u32 * rows_per_band + j as u32 / width;
            let pixel = rgb.get_pixel(x, y);
            *b = 0xFF00_0000 | (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | (pixel[2] as u32);
        }
    });

    Ok(ImgBuf {
        buf,
        width: width as usize,
        height: height as usize,
    })
}

fn new_window(size: WindowSize) -> Window {
    Window::new(
        "slide-show-rs - Press ESC to exit",
        size.0,
        size.1,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::Center,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to open Window")
}

fn main() {
    simple_logger::init().unwrap();

    // get window size
    let w = dotenv::var("WINDOW_WIDTH");
    let h = dotenv::var("WINDOW_HEIGHT");
    let size = WindowSize(
        match w {
            Ok(w_str) => w_str.parse().unwrap(),
            Err(_) => 640,
        },
        match h {
            Ok(h_str) => h_str.parse().unwrap(),
            Err(_) => 480,
        },
    );

    let args: Vec<String> = env::args().collect();
    let dir = if args.len() < 2 { "./photo" } else { &args[1] };
    let mut img_filepaths = match get_scaled_img_filepath_array(&dir, size) {
        Ok(a) => a,
        // unrecoverable: given directory itself has problem
        Err(ImageFilepathError::InvalidDirectory) => panic!("Invalid directory path: {}", dir),
        Err(ImageFilepathError::InvalidCharset) => panic!("Directory path isn't based on UTF-8"),
        Err(ImageFilepathError::InvalidGlobPattern(e)) => panic!(e.msg),
        Err(ImageFilepathError::NoImageFileFound) => panic!("No image file found in {}", dir),
    };
    info!("images found: {:?}", img_filepaths);
    let mut rng = rand::thread_rng();
    img_filepaths.shuffle(&mut rng);

    // load first image before opening window
    // `img.filepaths.first()` never fail because empty error is inspected above
    let mut img_buf = image_buffer_from_filepath(img_filepaths.first().unwrap()).unwrap();

    let mut window = new_window(size);
    let mut img_idx = 0;
    let mut interval_sec: f32 = 5.0;
    let mut start_time = SystemTime::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::Up, KeyRepeat::No)
            || window.is_key_pressed(Key::Right, KeyRepeat::No)
        {
            interval_sec -= 0.5;
            info!("Speed up: {}s", interval_sec)
        } else if window.is_key_pressed(Key::Down, KeyRepeat::No)
            || window.is_key_pressed(Key::Left, KeyRepeat::No)
        {
            interval_sec += 0.5;
            info!("Speed down: {}s", interval_sec);
        }
        if start_time.elapsed().unwrap() >= Duration::from_secs_f32(interval_sec) {
            img_idx = (img_idx + 1) % img_filepaths.len();
            // if error is detected, skip its image and try to read next one
            img_buf = match image_buffer_from_filepath(&img_filepaths[img_idx]) {
                Ok(i) => i,
                Err(_) => {
                    warn!("Failed to read image {}", img_filepaths[img_idx].display());
                    continue;
                }
            };
            start_time = SystemTime::now();
        }
        match window.update_with_buffer(&img_buf.buf, img_buf.width, img_buf.height) {
            Ok(_) => {}
            Err(e) => panic!("{}", e),
        }
    }
}
