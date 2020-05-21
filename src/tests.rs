use super::*;

#[test]
fn test_new_window() {
    let size = WindowSize(40, 20);
    let window = new_window(size);
    let ws = window.get_size();
    assert_eq!(size.0, ws.0);
    assert_eq!(size.1, ws.1);
}

#[test]
// TODO: better to have another (small) image directory to test.
//       to archieve it, first, add argument to `get_scaled_img_filepath_array`.
// TODO: make sure that huge image is resized to fit given size
fn test_img_filepath() {
    // ensure all image paths are included
    let img_filepaths = get_scaled_img_filepath_array(WindowSize(3840, 2160)).unwrap();
    for entry in glob("photo/*.jpg").unwrap() {
        let filename = &entry.unwrap().to_str().unwrap().replace("photo/", "").replace("resized/", "");
        assert!(img_filepaths.iter().any(|v| v.find(filename) != None));
    }
}
