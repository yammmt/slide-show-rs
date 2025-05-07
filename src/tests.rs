use super::*;

#[test]
// TODO: make sure that huge image is resized to fit given size
fn test_img_filepath() {
    // failure cases
    let err1 = get_scaled_img_filepath_array("./.gitignore", WindowSize(3840, 2160)).unwrap_err();
    assert!(matches!(err1, ImageFilepathError::InvalidDirectory));

    let err2 = get_scaled_img_filepath_array("./src", WindowSize(20, 10)).unwrap_err();
    assert!(matches!(err2, ImageFilepathError::NoImageFileFound));

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
    let err = get_scaled_img_filepath_array("./photo/test/*.jpg", WindowSize(3840, 2160)).unwrap_err();
    assert!(matches!(err, ImageFilepathError::InvalidDirectory));
}

#[test]
fn test_img_buffer() {
    let (tx1, rx1) = mpsc::channel();
    let (tx2, rx2) = mpsc::channel();
    rayon::spawn(move || {
        image_buffer_from_filepath(tx1, rx2);
    });

    tx2.send(ThreadMessage::Filepath("photo/test/sawayaka256.jpg"))
        .unwrap();
    match rx1.recv().unwrap() {
        ThreadMessage::ImageBuffer(ib) => {
            if let Ok(img_buf) = ib {
                // there are NO practical ways to test image buffer itself...
                assert_eq!(img_buf.width, 256);
                assert_eq!(img_buf.height, 256);
            } else {
                panic!();
            }
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
            // TODO: make sure that error is `ImageBufferError::OpenError` type
            assert!(ib.is_err());
        }
        _ => panic!(),
    }

    tx2.send(ThreadMessage::Close).unwrap();
    rx1.recv().unwrap();
}
