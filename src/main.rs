extern crate glob;
extern crate image;

use dotenv;
use glob::glob;
use image::{imageops, GenericImageView};
use minifb::{Key, ScaleMode, Window, WindowOptions};
use std::path::Path;
use std::time::{Duration, SystemTime};

#[cfg(test)] mod tests;

struct ImgBuf {
    pub buf: Vec<u32>,
    pub width: usize,
    pub height: usize,
}
#[derive(Copy, Clone, Debug)]
struct WindowSize(usize, usize);

// TODO: allow any glob pattern as argument
fn get_scaled_img_filepath_array(window_size: WindowSize) -> Result<Vec<String>, String> {
    let pattern = "photo/*.jpg";
    let mut img_filepaths: Vec<String> = vec![];
    for entry in glob(&pattern).expect("Failed to read glob pattern") {
        if let Ok(path) = entry {
            if let Some(s) = path.to_str() {
                let mut img = image::open(s).unwrap();
                if img.dimensions().0 > window_size.0 as u32 || img.dimensions().1 > window_size.1 as u32 {
                    // resize big images to load fast
                    // for resize algorithm detail, see official documents at
                    // https://docs.rs/image/0.23.4/image/imageops/enum.FilterType.html#examples
                    img = img.resize(window_size.0 as u32, window_size.1 as u32, imageops::CatmullRom);
                    let resized_filename = s.replace("photo/", "photo/resized/");
                    img.save(&resized_filename).unwrap();
                    img_filepaths.push(resized_filename)
                } else {
                    img_filepaths.push(s.to_string());
                }
            } else {
                println!("Failed to parse path {:?}", path);
            }
        }
    }
    if img_filepaths.len() > 0 {
        Ok(img_filepaths)

    } else {
        Err(format!("No image files found in {}", pattern))
    }
}

// TODO: return `Result` to run recovery process
fn image_buffer_from_filepath<P>(filepath: P) -> ImgBuf
    where P: AsRef<Path> {
    let img = image::open(&filepath).unwrap();
    let rgb = img.as_rgb8().unwrap();

    let (width, height) = rgb.dimensions();
    let mut buf: Vec<u32> = vec![];
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            buf.push(0xFF000000 | (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | (pixel[2] as u32));
        }
    }
    ImgBuf{ buf, width: width as usize, height: height as usize }
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
            Err(_) => 640
        },
        match h {
            Ok(h_str) => h_str.parse().unwrap(),
            Err(_) => 480
        },
    );

    let img_filepaths = get_scaled_img_filepath_array(size).unwrap();
    println!("images found: {:?}", img_filepaths);

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
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
    }
}
