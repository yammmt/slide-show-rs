extern crate image;

use minifb::{Key, ScaleMode, Window, WindowOptions};

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

    let mut window = Window::new(
        "slide-show-rs - Press ESC to exit",
        width as usize,
        height as usize,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::Center,
            ..WindowOptions::default()
        },
    )
    .expect("Unable to open Window");

    while window.is_open() && !window.is_key_down(Key::Escape) {
        match window.update_with_buffer(&buf, width as usize, height as usize) {
            Ok(_) => {},
            Err(e) => panic!("{}", e),
        }
    }
}
