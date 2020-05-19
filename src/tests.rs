use super::*;
use std::path::Path;

#[test]
fn test_new_window() {
    let env_exists = Path::new(".env").exists();

    match env_exists {
        true => {
            // with env
            let w = new_window();
            let size = w.get_size();
            assert_eq!(size.0, dotenv::var("WINDOW_WIDTH").unwrap().parse::<usize>().unwrap());
            assert_eq!(size.1, dotenv::var("WINDOW_HEIGHT").unwrap().parse::<usize>().unwrap());
        },
        false => {
            // without env
            let w = new_window();
            let size = w.get_size();
            assert_eq!(size.0, 640);
            assert_eq!(size.1, 480);
        },
    }
}
