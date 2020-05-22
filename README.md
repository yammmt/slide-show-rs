# slide-show-rs

Show images

## Usage

Before compiling, make sure that your window size is defined in `.env` file. For example,

```text
WINDOW_WIDTH=1920
WINDOW_HEIGHT=1080
```

To reduce time to build, use `--release` option. It makes this program much faster.

```bash
cargo run --release
```

If you got error(s), please confirm support status of minifb crate ([repo](https://github.com/emoon/rust_minifb)).

## Test

To avoid `NSInternalInconsistencyException`, test **must** be run with `--test_threads=1` option.
That is, run `cargo test -- --test-threads=1`.

## Links

The idea of image viewer comes from:

- [rust_minifb image example](https://github.com/emoon/rust_minifb/blob/master/examples/image.rs)
- [Issue in rust_minifb reop](https://github.com/emoon/rust_minifb/issues/48)
