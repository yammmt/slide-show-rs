extern crate glob;
extern crate image;

use glob::glob;
use image::{imageops, GenericImageView};
use minifb::{Key, ScaleMode, Window, WindowOptions};
use rand::seq::SliceRandom;
use std::env;
use std::fmt;
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

#[derive(Debug, PartialEq)]
enum ImageFilepathError<P: AsRef<Path>> {
    InvalidDirectory(P),
    InvalidCharset,
    InvalidGlobPattern(P),
    NoImageFileFound(P),
}

impl<P> fmt::Display for ImageFilepathError<P>
where
    P: AsRef<Path>,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ImageFilepathError::InvalidDirectory(ref dir) => {
                write!(f, "Invalid directory path: {}", dir.as_ref().display())
            }
            ImageFilepathError::InvalidCharset => write!(f, "Use UTF-8 for photo directory name"),
            ImageFilepathError::InvalidGlobPattern(ref dir) => {
                write!(f, "Invalid glob pattern: {}", dir.as_ref().display())
            }
            ImageFilepathError::NoImageFileFound(ref dir) => {
                write!(f, "No image file found in {}", dir.as_ref().display())
            }
        }
    }
}

fn get_scaled_img_filepath_array<P>(
    dir: P,
    window_size: WindowSize,
) -> Result<Vec<PathBuf>, ImageFilepathError<P>>
where
    P: AsRef<Path>,
{
    if !dir.as_ref().is_dir() {
        return Err(ImageFilepathError::InvalidDirectory(dir));
    }

    let filepath_base = dir.as_ref().join("*.jpg");
    let pat = match filepath_base.to_str() {
        Some(p) => p,
        None => return Err(ImageFilepathError::InvalidCharset),
    };

    let glob_pat = match glob(pat) {
        Ok(g) => g,
        Err(_) => return Err(ImageFilepathError::InvalidGlobPattern(dir)),
    };

    let mut img_filepaths: Vec<PathBuf> = vec![];
    for entry in glob_pat {
        if let Ok(s) = entry {
            let s_file_name = match s.file_name() {
                Some(n) => n,
                None => {
                    println!("Failed to get filename of {}", s.display());
                    continue;
                }
            };

            let mut img = match image::open(&s) {
                Ok(i) => i,
                Err(_) => {
                    println!("Failed to read image {}", s.display());
                    continue;
                }
            };

            if img.dimensions().0 > window_size.0 as u32
                || img.dimensions().1 > window_size.1 as u32
            {
                // resize big images to load fast
                // for resize algorithm detail, see official documents at
                // https://docs.rs/image/0.23.4/image/imageops/enum.FilterType.html#examples
                img = img.resize(
                    window_size.0 as u32,
                    window_size.1 as u32,
                    imageops::CatmullRom,
                );
                let resized_filename = s
                    .parent()
                    .unwrap_or_else(|| panic!("Failed to get parent directory of {:?}", s))
                    .join("resized")
                    .join(s_file_name);
                match img.save(&resized_filename) {
                    Ok(_) => img_filepaths.push(resized_filename),
                    Err(_) => println!("Failed to save {}", resized_filename.display()),
                };
            } else {
                img_filepaths.push(PathBuf::from(&s));
            }
        } else {
            println!("Failed to parse path {:?}", entry);
        }
    }
    if !img_filepaths.is_empty() {
        Ok(img_filepaths)
    } else {
        Err(ImageFilepathError::NoImageFileFound(dir))
    }
}

// TODO: return `Result` to run recovery process
fn image_buffer_from_filepath<P>(filepath: P) -> ImgBuf
where
    P: AsRef<Path>,
{
    let img = image::open(&filepath).unwrap();
    let rgb = img.as_rgb8().unwrap();

    let (width, height) = rgb.dimensions();
    let mut buf: Vec<u32> = vec![];
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            buf.push(
                0xFF00_0000 | (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | (pixel[2] as u32),
            );
        }
    }
    ImgBuf {
        buf,
        width: width as usize,
        height: height as usize,
    }
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
    let mut img_filepaths = get_scaled_img_filepath_array(&dir, size).unwrap();
    println!("images found: {:?}", img_filepaths);
    let mut rng = rand::thread_rng();
    img_filepaths.shuffle(&mut rng);

    // load first image before opening window
    let mut img_buf = image_buffer_from_filepath(img_filepaths.first().unwrap());

    let mut window = new_window(size);
    let mut img_idx = 0;
    let mut start_time = SystemTime::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if start_time.elapsed().unwrap() >= Duration::from_secs(5) {
            img_idx = (img_idx + 1) % img_filepaths.len();
            // TODO: if error is detected, skip its image and try to read next one
            img_buf = image_buffer_from_filepath(&img_filepaths[img_idx]);
            start_time = SystemTime::now();
        }
        match window.update_with_buffer(&img_buf.buf, img_buf.width, img_buf.height) {
            Ok(_) => {}
            Err(e) => panic!("{}", e),
        }
    }
}
