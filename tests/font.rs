use ghe2d::font::LoadFont;
use ghe2d::Ghe2dError;

#[test]
fn missing_font_file_errors_instead_of_panicking() {
    let r = LoadFont::new("does/not/exist-abc123.ttf".to_string());
    assert!(matches!(r, Err(Ghe2dError::Io(_))), "got {r:?}");
}

#[test]
fn invalid_font_bytes_error() {
    let r = LoadFont::from_bytes(vec![0, 1, 2, 3, 4, 5, 6, 7]);
    assert!(matches!(r, Err(Ghe2dError::FontLoad)), "got {r:?}");
}
