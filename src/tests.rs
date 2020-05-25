use super::*;

#[test]
// TODO: make sure that huge image is resized to fit given size
fn test_img_filepath() {
    // failure cases
    // TODO: are there better ways to test returned type?
    match get_scaled_img_filepath_array("./.gitignore", WindowSize(3840, 2160)) {
        Ok(_) => panic!(),
        Err(e) => assert_eq!(e, ImageFilepathError::InvalidDirectory("./.gitignore")),
    }

    match get_scaled_img_filepath_array("./src", WindowSize(20, 10)) {
        Ok(_) => panic!(),
        Err(e) => assert_eq!(e, ImageFilepathError::NoImageFileFound("./src")),
    }

    // success case
    // ensure all image paths are included
    let img_filepaths =
        get_scaled_img_filepath_array("./photo/test/", WindowSize(3840, 2160)).unwrap();
    for entry in glob("photo/test/*.jpg").unwrap() {
        let e = entry.unwrap();
        let filename = e.file_name().unwrap().to_str().unwrap();
        if filename == "dummy.jpg" {
            // dummy file (without contents) must be skipped
            continue;
        }
        assert!(img_filepaths.iter().any(|v| v.ends_with(filename)));
    }
    assert!(!img_filepaths.iter().any(|v| v.ends_with("dummy.jpg")));
}

#[test]
fn test_img_filepath_with_wrong_dir() {
    assert!(get_scaled_img_filepath_array("./photo/test/*.jpg", WindowSize(3840, 2160)).is_err());
}

#[test]
fn test_img_buffer() {
    let img_buf = image_buffer_from_filepath("photo/test/sawayaka256.jpg");
    // there are NO practical ways to test image buffer itself...
    assert_eq!(img_buf.width, 256);
    assert_eq!(img_buf.height, 256);
}

#[test]
#[should_panic]
fn test_img_buffer_with_wrong_filepath() {
    let _ = image_buffer_from_filepath(".gitignore");
}
