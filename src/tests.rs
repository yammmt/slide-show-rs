use super::*;

#[test]
fn test_get_scaled_img_filepath_array_invalid_directory() {
    let err = get_scaled_img_filepath_array("./.gitignore", WindowSize(3840, 2160)).unwrap_err();
    assert!(matches!(err, ImageFilepathError::InvalidDirectory));

    let err = get_scaled_img_filepath_array("./test/assets/photo/*.jpg", WindowSize(3840, 2160))
        .unwrap_err();
    assert!(matches!(err, ImageFilepathError::InvalidDirectory));
}

#[test]
fn test_get_scaled_img_filepath_array_no_image_file_found() {
    let err = get_scaled_img_filepath_array("./src", WindowSize(20, 10)).unwrap_err();
    assert!(matches!(err, ImageFilepathError::NoImageFileFound));
}

#[test]
fn test_get_scaled_img_filepath_array_success_case() {
    // ensure all image paths are included
    let img_filepaths =
        get_scaled_img_filepath_array("./test/assets/photo/", WindowSize(3840, 2160)).unwrap();
    for entry in glob("test/assets/photo/*.jpg").unwrap() {
        let e = entry.unwrap();
        let filename = e.file_name().unwrap().to_str().unwrap();
        if filename == "dummy.jpg" {
            // dummy file (without contents) must be skipped
            continue;
        }
        assert!(
            img_filepaths.iter().any(|v| v.ends_with(filename)),
            "Expected file {filename} not found in img_filepaths: {img_filepaths:?}"
        );
    }
    assert!(!img_filepaths.iter().any(|v| v.ends_with("dummy.jpg")));
}

#[test]
fn test_img_buffer() {
    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    rayon::spawn(move || {
        image_buffer_from_filepath(tx1, rx2);
    });

    tx2.send(ThreadMessage::Filepath("test/assets/photo/sawayaka256.jpg"))
        .unwrap();
    match rx1.recv().unwrap() {
        ThreadMessage::ImageBuffer(ib) => {
            // Only basic properties (like dimensions) are checked here, as
            // verifying the entire image buffer content is impractical in tests.
            let img_buf = ib.expect("Expected Ok(ImageBuffer), got error");
            assert_eq!(img_buf.width, 256);
            assert_eq!(img_buf.height, 256);
        }
        _ => panic!(),
    }

    tx2.send(ThreadMessage::Close).unwrap();
    rx1.recv().unwrap();
}

#[test]
fn test_img_buffer_with_wrong_filepath() {
    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    rayon::spawn(move || {
        image_buffer_from_filepath(tx1, rx2);
    });

    tx2.send(ThreadMessage::Filepath(".gitignore")).unwrap();
    match rx1.recv().unwrap() {
        ThreadMessage::ImageBuffer(ib) => {
            assert!(matches!(ib, Err(ImageBufferError::OpenError(_))));
        }
        _ => panic!(),
    }

    tx2.send(ThreadMessage::Close).unwrap();
    rx1.recv().unwrap();
}
