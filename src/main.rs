extern crate image;

use dotenv;
use minifb::{Key, ScaleMode, Window, WindowOptions};

#[cfg(test)] mod tests;

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
    let filepath = "photo/sawayaka256.jpg";
    let img = image::open(&filepath).unwrap();
    let rgb = img.as_rgb8().unwrap();

    let (width, height) = rgb.dimensions();
    let mut buf: Vec<u32> = vec![] ;
    for y in 0..height {
        for x in 0..width {
            let pixel = rgb.get_pixel(x as u32, y as u32);
            buf.push(0xFF000000 | (pixel[0] as u32) << 16 | (pixel[1] as u32) << 8 | (pixel[2] as u32));
        }
    }

    let mut window = new_window();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        match window.update_with_buffer(&buf, width as usize, height as usize) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
    }
}
