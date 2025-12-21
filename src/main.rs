use glob::glob;
use image::{GenericImageView, imageops};
use log::{info, warn};
use minifb::{Key, KeyRepeat, ScaleMode, Window, WindowOptions};
use rand::seq::SliceRandom;
use rayon::prelude::*;
use simple_logger::SimpleLogger;
use std::env;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::mpsc;
use std::time::{Duration, SystemTime};

#[cfg(test)]
mod tests;

#[derive(Clone)]
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

// couldn't use `#[derive(Debug)]` because of `ImgBuf`
enum ThreadMessage<P>
where
    P: AsRef<Path>,
{
    Filepath(P),
    ImageBuffer(Result<ImgBuf, ImageBufferError>),
    Close,
}

impl fmt::Display for ImageFilepathError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &self {
            ImageFilepathError::InvalidDirectory => write!(f, "Invalid directory path"),
            ImageFilepathError::InvalidCharset => write!(f, "Use UTF-8 for photo directory name"),
            ImageFilepathError::InvalidGlobPattern(e) => e.fmt(f),
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
    let begin_time = SystemTime::now();
    if !dir.as_ref().is_dir() {
        return Err(ImageFilepathError::InvalidDirectory);
    }

    let filepath_base = dir.as_ref().join("*.jpg");
    let pat = match filepath_base.to_str() {
        Some(p) => p,
        None => return Err(ImageFilepathError::InvalidCharset),
    };
    let glob_pat: Vec<Result<PathBuf, _>> = glob(pat)?.collect();

    let img_filepaths: Vec<PathBuf> = glob_pat
        .into_par_iter()
        .filter_map(|entry| {
            if let Ok(s) = entry {
                let s_file_name = match s.file_name() {
                    Some(n) => n,
                    None => {
                        warn!("Failed to get filename of {}", s.display());
                        return None;
                    }
                };

                let mut img = match image::open(&s) {
                    Ok(i) => i,
                    Err(_) => {
                        warn!("Failed to read image {}", s.display());
                        return None;
                    }
                };

                let resized_dir = s
                    .parent()
                    .unwrap_or_else(|| panic!("Failed to get parent directory of {s:?}"))
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
                        Ok(_) => Some(resized_filename),
                        Err(_) => {
                            warn!("Failed to save {}", resized_filename.display());
                            None
                        }
                    }
                } else {
                    Some(PathBuf::from(&s))
                }
            } else {
                warn!("Failed to parse path {entry:?}");
                None
            }
        })
        .collect();
    if !img_filepaths.is_empty() {
        info!(
            "Time for image preprocessing: {:?}",
            begin_time.elapsed().unwrap()
        );
        Ok(img_filepaths)
    } else {
        Err(ImageFilepathError::NoImageFileFound)
    }
}

fn image_buffer_from_filepath<P>(
    tx: mpsc::Sender<ThreadMessage<P>>,
    rx: mpsc::Receiver<ThreadMessage<P>>,
) where
    P: AsRef<Path>,
{
    loop {
        let received_data = rx.recv().unwrap();
        let filepath = match received_data {
            ThreadMessage::Filepath(p) => p,
            ThreadMessage::Close => {
                tx.send(ThreadMessage::Close).unwrap();
                return;
            }
            _ => panic!("Received invalid message from main"),
        };

        let img = match image::open(&filepath) {
            Ok(i) => i,
            Err(e) => {
                tx.send(ThreadMessage::ImageBuffer(Err(ImageBufferError::from(e))))
                    .unwrap();
                continue;
            }
        };
        let rgb = match img.as_rgb8() {
            Some(r) => r,
            None => {
                tx.send(ThreadMessage::ImageBuffer(Err(
                    ImageBufferError::RgbParseError,
                )))
                .unwrap();
                continue;
            }
        };

        let (width, height) = rgb.dimensions();
        let mut buf: Vec<u32> = vec![0; (width * height) as usize];

        let threads =
            std::thread::available_parallelism().expect("Failed to get available parallelism num");
        let rows_per_band = height / threads.get() as u32 + 1;
        let bands: Vec<&mut [u32]> = buf.chunks_mut((rows_per_band * width) as usize).collect();
        bands.into_par_iter().enumerate().for_each(|(i, band)| {
            for (j, b) in band.iter_mut().enumerate() {
                let x = j as u32 % width;
                let y = i as u32 * rows_per_band + j as u32 / width;
                let pixel = rgb.get_pixel(x, y);
                *b = 0xFF00_0000
                    | (pixel[0] as u32) << 16
                    | (pixel[1] as u32) << 8
                    | (pixel[2] as u32);
            }
        });

        tx.send(ThreadMessage::ImageBuffer(Ok(ImgBuf {
            buf,
            width: width as usize,
            height: height as usize,
        })))
        .unwrap()
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
    SimpleLogger::new().init().unwrap();

    // get window size
    let w = dotenvy::var("WINDOW_WIDTH");
    let h = dotenvy::var("WINDOW_HEIGHT");
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
    let mut img_filepaths = match get_scaled_img_filepath_array(dir, size) {
        Ok(a) => a,
        // unrecoverable: given directory itself has problem
        Err(ImageFilepathError::InvalidDirectory) => panic!("Invalid directory path: {dir}"),
        Err(ImageFilepathError::InvalidCharset) => panic!("Directory path isn't based on UTF-8"),
        Err(ImageFilepathError::InvalidGlobPattern(e)) => panic!("{}", e.msg),
        Err(ImageFilepathError::NoImageFileFound) => panic!("No image file found in {dir}"),
    };
    info!("images found: {img_filepaths:?}");
    let mut rng = rand::rng();
    img_filepaths.shuffle(&mut rng);

    // use threads to prepare image buffer before it is needed
    let (tx_to_main, rx_from_buf_func) = mpsc::channel();
    let (tx_to_buf_func, rx_from_main) = mpsc::channel();
    rayon::spawn(move || {
        image_buffer_from_filepath(tx_to_main, rx_from_main);
    });

    // load first image before opening window
    let mut img_buf: ImgBuf;
    let mut img_idx = 0;
    tx_to_buf_func
        .send(ThreadMessage::Filepath(img_filepaths[img_idx].clone()))
        .unwrap();
    loop {
        let res = rx_from_buf_func.recv().unwrap();
        match res {
            ThreadMessage::ImageBuffer(img_buf_result) => match img_buf_result {
                Ok(ib) => {
                    img_buf = ib;
                    img_idx = (img_idx + 1) % img_filepaths.len();
                    tx_to_buf_func
                        .send(ThreadMessage::Filepath(img_filepaths[img_idx].clone()))
                        .unwrap();
                    break;
                }
                Err(_) => {
                    img_idx = (img_idx + 1) % img_filepaths.len();
                    if img_idx == 0 {
                        panic!("Failed to read all of found images");
                    }
                    tx_to_buf_func
                        .send(ThreadMessage::Filepath(img_filepaths[img_idx].clone()))
                        .unwrap();
                }
            },
            _ => panic!("Received invalid message from image buffer function"),
        }
    }

    let mut window = new_window(size);
    let mut interval_sec: f32 = 5.0;
    let mut next_img_buf = ImgBuf {
        buf: vec![0; 1],
        width: 1,
        height: 1,
    };
    let mut start_time = SystemTime::now();
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if window.is_key_pressed(Key::Up, KeyRepeat::No)
            || window.is_key_pressed(Key::Right, KeyRepeat::No)
        {
            interval_sec = 0.5f32.max(interval_sec - 0.5);
            info!("Speed up: {interval_sec}s")
        } else if window.is_key_pressed(Key::Down, KeyRepeat::No)
            || window.is_key_pressed(Key::Left, KeyRepeat::No)
        {
            interval_sec += 0.5;
            info!("Speed down: {interval_sec}s");
        }

        // If receiving buffer here, we have to `clone` this value again when
        // setting time passes, in the following process. However, it's good to
        // make full use of free time to read the next image buffer.
        if let Ok(res) = rx_from_buf_func.try_recv() {
            match res {
                ThreadMessage::ImageBuffer(img_buf_result) => match img_buf_result {
                    Ok(ib) => next_img_buf = ib,
                    Err(e) => {
                        // try to read next image of next image
                        info!("error: {e:?}");
                        img_idx = (img_idx + 1) % img_filepaths.len();
                        tx_to_buf_func
                            .send(ThreadMessage::Filepath(img_filepaths[img_idx].clone()))
                            .unwrap();
                    }
                },
                _ => panic!("Received invalid message from image buffer function"),
            }
        };

        if start_time.elapsed().unwrap() >= Duration::from_secs_f32(interval_sec) {
            img_idx = (img_idx + 1) % img_filepaths.len();
            img_buf = next_img_buf.clone();
            tx_to_buf_func
                .send(ThreadMessage::Filepath(img_filepaths[img_idx].clone()))
                .unwrap();
            start_time = SystemTime::now();
        }
        match window.update_with_buffer(&img_buf.buf, img_buf.width, img_buf.height) {
            Ok(_) => {}
            Err(e) => panic!("{}", e),
        }
    }

    // close thread
    tx_to_buf_func.send(ThreadMessage::Close).unwrap();
    rx_from_buf_func.recv().unwrap();
}
