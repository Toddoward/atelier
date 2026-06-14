//! Raster image decode/encode for Place and Export. Wraps the `image` crate;
//! decodes/encodes PNG, JPEG, TIFF, WebP, GIF, BMP (spec 0032/0033/0034).

/// Import file extensions accepted by Place / Open dialogs.
pub const IMPORT_EXTENSIONS: &[&str] = &["png", "jpg", "jpeg", "tiff", "tif", "webp", "gif", "bmp"];
/// Export file extensions offered (lossless/alpha-friendly first).
pub const EXPORT_EXTENSIONS: &[&str] = &["png", "tiff", "tif", "bmp", "webp", "jpg", "jpeg"];

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported or corrupt image: {0}")]
    Decode(#[from] image::ImageError),
    #[error("rgba buffer does not match {0}x{1}")]
    Buffer(u32, u32),
}

/// A decoded image: `width × height` straight-alpha RGBA8, row-major.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecodedImage {
    pub width: u32,
    pub height: u32,
    pub rgba: Vec<u8>,
}

/// Decode encoded image bytes (PNG/JPEG) to RGBA8.
pub fn decode_image(bytes: &[u8]) -> Result<DecodedImage, ImageError> {
    let img = image::load_from_memory(bytes)?.to_rgba8();
    let (width, height) = (img.width(), img.height());
    Ok(DecodedImage { width, height, rgba: img.into_raw() })
}

/// Decode an image file from disk.
pub fn load_image(path: &std::path::Path) -> Result<DecodedImage, ImageError> {
    decode_image(&std::fs::read(path)?)
}

/// Encode a straight-alpha RGBA8 buffer to PNG bytes.
pub fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, ImageError> {
    if rgba.len() != (width as usize * height as usize * 4) {
        return Err(ImageError::Buffer(width, height));
    }
    let mut out = Vec::new();
    use image::ImageEncoder;
    image::codecs::png::PngEncoder::new(&mut out)
        .write_image(rgba, width, height, image::ExtendedColorType::Rgba8)?;
    Ok(out)
}

/// Save a flattened RGBA8 buffer to `path`, choosing the format from the
/// extension (JPEG drops alpha onto an RGB image; everything else stays RGBA).
pub fn save_image(
    path: &std::path::Path,
    width: u32,
    height: u32,
    rgba: &[u8],
) -> Result<(), ImageError> {
    let buf = image::RgbaImage::from_raw(width, height, rgba.to_vec())
        .ok_or(ImageError::Buffer(width, height))?;
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("png").to_ascii_lowercase();
    if ext == "jpg" || ext == "jpeg" {
        image::DynamicImage::ImageRgba8(buf).to_rgb8().save(path)?;
    } else {
        buf.save(path)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Encode a tiny RGBA image to PNG, decode it back, expect the same pixels.
    #[test]
    fn png_round_trip() {
        let (w, h) = (3u32, 2u32);
        let mut rgba = Vec::new();
        for i in 0..(w * h) {
            rgba.extend_from_slice(&[i as u8 * 10, 20, 30, 255]);
        }
        let mut png = Vec::new();
        {
            let enc = image::codecs::png::PngEncoder::new(&mut png);
            use image::ImageEncoder;
            enc.write_image(&rgba, w, h, image::ExtendedColorType::Rgba8).unwrap();
        }
        let got = decode_image(&png).unwrap();
        assert_eq!((got.width, got.height), (w, h));
        assert_eq!(got.rgba, rgba);
    }

    #[test]
    fn garbage_bytes_error_not_panic() {
        assert!(decode_image(b"not an image").is_err());
    }

    #[test]
    fn encode_png_round_trips_through_decode() {
        let (w, h) = (4u32, 4u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|i| [i as u8, 0, 255, 255]).collect();
        let png = encode_png(w, h, &rgba).unwrap();
        let got = decode_image(&png).unwrap();
        assert_eq!((got.width, got.height), (w, h));
        assert_eq!(got.rgba, rgba);
    }

    #[test]
    fn save_image_writes_a_readable_png() {
        let (w, h) = (2u32, 2u32);
        let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 9, 9, 9, 255];
        let path = std::env::temp_dir().join(format!("atelier-export-{}.png", std::process::id()));
        save_image(&path, w, h, &rgba).unwrap();
        let got = load_image(&path).unwrap();
        std::fs::remove_file(&path).ok();
        assert_eq!((got.width, got.height), (w, h));
        assert_eq!(&got.rgba[0..4], &[255, 0, 0, 255]);
    }

    #[test]
    fn encode_png_rejects_mismatched_buffer() {
        assert!(matches!(encode_png(4, 4, &[0, 0, 0, 255]), Err(ImageError::Buffer(4, 4))));
    }

    /// Lossless formats round-trip through save_image/load_image.
    #[test]
    fn lossless_formats_round_trip() {
        let (w, h) = (3u32, 2u32);
        let rgba: Vec<u8> = (0..w * h).flat_map(|i| [i as u8 * 8, 40, 200, 255]).collect();
        for ext in ["tiff", "bmp"] {
            let path = std::env::temp_dir()
                .join(format!("atelier-fmt-{}-{ext}.{ext}", std::process::id()));
            save_image(&path, w, h, &rgba).unwrap();
            let got = load_image(&path).unwrap();
            std::fs::remove_file(&path).ok();
            assert_eq!((got.width, got.height), (w, h), "{ext} size");
            assert_eq!(got.rgba, rgba, "{ext} pixels round-trip");
        }
    }
}
