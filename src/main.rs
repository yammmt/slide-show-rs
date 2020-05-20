extern crate glob;
extern crate image;

use dotenv;
use glob::glob;
use minifb::{Key, ScaleMode, Window, WindowOptions};
use std::time::{Duration, SystemTime};

#[cfg(test)] mod tests;

struct ImgBuf {
    pub buf: Vec<u32>,
    pub width: usize,
    pub height: usize,
}
#[derive(Debug)]
struct WindowSize(usize, usize);

fn new_window() -> Window {
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
    // get image file paths
    let mut img_filepaths: Vec<String> = vec![];
    for entry in glob("./photo/*.jpg").expect("Failed to read glob pattern") {
        match entry {
            Ok(path) => {
                match path.to_str() {
                    Some(s) => img_filepaths.push(s.to_string()),
                    None => {},
                }
            }
            Err(_) => {},
        }
    }
    // TODO: use logger?
    println!("{:?}", img_filepaths);
    if img_filepaths.len() == 0 {
        println!("No .jpg file found.");
        return;
    }

    // load images before opening window
    let mut img_bufs: Vec<_> = vec![];
    for i in &img_filepaths {
        let img = image::open(&i).unwrap();
        let rgb = img.as_rgb8().unwrap();

        let (width, height) = rgb.dimensions();
        let mut buf: Vec<u32> = vec![] ;
        // TODO: better to use thread?
        for y in 0..height {
            for x in 0..width {
                let pixel = rgb.get_pixel(x as u32, y as u32);
                buf.push(0xFF000000 | (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | (pixel[2] as u32));
            }
        }
        img_bufs.push(ImgBuf{ buf, width: width as usize, height: height as usize });
    }

    let mut window = new_window();
    let mut img_idx = 0;
    let mut img_buf = &img_bufs[img_idx];
    let mut start_time = SystemTime::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if start_time.elapsed().unwrap() >= Duration::from_secs(5) {
            img_idx = (img_idx + 1) % img_bufs.len();
            img_buf = &img_bufs[img_idx];
            start_time = SystemTime::now();
        }
        match window.update_with_buffer(&img_buf.buf, img_buf.width, img_buf.height) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
    }
}
