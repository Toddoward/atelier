//! Raster image decoding for Place (INT-3, spec 0032). Wraps the `image` crate;
//! decodes PNG/JPEG bytes to straight-alpha RGBA8.

#[derive(Debug, thiserror::Error)]
pub enum ImageError {
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("unsupported or corrupt image: {0}")]
    Decode(#[from] image::ImageError),
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
}
