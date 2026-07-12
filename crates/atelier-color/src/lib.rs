//! Color management (lcms2 wrapper). All color conversions live here — no
//! ad-hoc color math outside this crate (architecture invariant).
//!
//! Spec 0059: profiles, RGBA8 buffer conversion, Lab readout for ΔE gates.

use lcms2::{Intent as LcmsIntent, PixelFormat, Profile as LcmsProfile, Transform};

#[derive(Debug, thiserror::Error)]
pub enum ColorError {
    #[error("invalid ICC profile: {0}")]
    BadProfile(String),
    #[error("cannot build transform: {0}")]
    BadTransform(String),
}

/// Rendering intent (subset we expose; maps 1:1 onto lcms2).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Intent {
    #[default]
    Perceptual,
    RelativeColorimetric,
    Saturation,
    AbsoluteColorimetric,
}

impl Intent {
    fn to_lcms(self) -> LcmsIntent {
        match self {
            Intent::Perceptual => LcmsIntent::Perceptual,
            Intent::RelativeColorimetric => LcmsIntent::RelativeColorimetric,
            Intent::Saturation => LcmsIntent::Saturation,
            Intent::AbsoluteColorimetric => LcmsIntent::AbsoluteColorimetric,
        }
    }
}

/// An ICC profile (owned lcms2 handle).
pub struct Profile(LcmsProfile);

impl Profile {
    /// Built-in sRGB (the document default working space).
    pub fn srgb() -> Self {
        Self(LcmsProfile::new_srgb())
    }

    /// Parse an ICC profile from raw bytes (e.g. embedded in a PNG/TIFF).
    pub fn from_icc(bytes: &[u8]) -> Result<Self, ColorError> {
        LcmsProfile::new_icc(bytes)
            .map(Self)
            .map_err(|e| ColorError::BadProfile(e.to_string()))
    }
}

/// Convert an interleaved RGBA8 buffer in place from `src` to `dst` space.
/// Alpha passes through untouched (lcms2 EXTRA channel).
pub fn convert_rgba8(
    pixels: &mut [u8],
    src: &Profile,
    dst: &Profile,
    intent: Intent,
) -> Result<(), ColorError> {
    let t: Transform<[u8; 4], [u8; 4]> = Transform::new(
        &src.0,
        PixelFormat::RGBA_8,
        &dst.0,
        PixelFormat::RGBA_8,
        intent.to_lcms(),
    )
    .map_err(|e| ColorError::BadTransform(e.to_string()))?;
    // Safety of the cast: RGBA8 is 4 bytes per pixel by construction.
    let px: &mut [[u8; 4]] = bytemuck_cast(pixels);
    t.transform_in_place(px);
    Ok(())
}

/// View a byte slice as [u8;4] pixels (len must be a multiple of 4).
fn bytemuck_cast(bytes: &mut [u8]) -> &mut [[u8; 4]] {
    assert_eq!(bytes.len() % 4, 0, "RGBA8 buffer length");
    // SAFETY: [u8;4] has the same layout/alignment as 4 consecutive u8s.
    unsafe { std::slice::from_raw_parts_mut(bytes.as_mut_ptr().cast(), bytes.len() / 4) }
}

/// sRGB (8-bit) → CIELAB via lcms2, for ΔE checks and picker readouts.
pub fn srgb_to_lab(rgb: [u8; 3]) -> [f32; 3] {
    let srgb = LcmsProfile::new_srgb();
    // D50 white point (ICC PCS) as xyY.
    let d50 = lcms2::CIExyY { x: 0.3457, y: 0.3585, Y: 1.0 };
    let lab = LcmsProfile::new_lab4_context(lcms2::GlobalContext::new(), &d50)
        .expect("built-in Lab profile");
    let t: Transform<[u8; 3], [f64; 3]> = Transform::new(
        &srgb,
        PixelFormat::RGB_8,
        &lab,
        PixelFormat::Lab_DBL,
        LcmsIntent::RelativeColorimetric,
    )
    .expect("sRGB→Lab transform");
    let mut out = [[0f64; 3]];
    t.transform_pixels(&[rgb], &mut out);
    [out[0][0] as f32, out[0][1] as f32, out[0][2] as f32]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn de76(a: [f32; 3], b: [f32; 3]) -> f32 {
        ((a[0] - b[0]).powi(2) + (a[1] - b[1]).powi(2) + (a[2] - b[2]).powi(2)).sqrt()
    }

    /// Published sRGB→Lab(D50) reference values (ICC PCS, 2° observer).
    #[test]
    fn srgb_primaries_hit_reference_lab_values() {
        // (rgb, expected Lab-D50) — values from standard sRGB→Lab tables.
        let cases: [([u8; 3], [f32; 3]); 4] = [
            ([255, 255, 255], [100.0, 0.0, 0.0]),
            ([255, 0, 0], [54.29, 80.81, 69.89]),
            ([0, 255, 0], [87.82, -79.29, 80.99]),
            ([0, 0, 255], [29.57, 68.30, -112.03]),
        ];
        for (rgb, want) in cases {
            let got = srgb_to_lab(rgb);
            assert!(
                de76(got, want) < 1.5,
                "ΔE76 too high for {rgb:?}: got {got:?}, want {want:?}"
            );
        }
    }

    /// sRGB→sRGB is identity within 1 LSB; alpha untouched.
    #[test]
    fn srgb_to_srgb_roundtrip_is_identity() {
        let mut px: Vec<u8> = vec![
            255, 0, 0, 255, //
            0, 255, 0, 128, //
            0, 0, 255, 0, //
            12, 34, 56, 200,
        ];
        let orig = px.clone();
        convert_rgba8(&mut px, &Profile::srgb(), &Profile::srgb(), Intent::default()).unwrap();
        for (i, (a, b)) in orig.iter().zip(&px).enumerate() {
            if i % 4 == 3 {
                assert_eq!(a, b, "alpha byte {i} must pass through exactly");
            } else {
                assert!((*a as i16 - *b as i16).abs() <= 1, "byte {i}: {a} vs {b}");
            }
        }
    }

    #[test]
    fn bad_icc_bytes_error_cleanly() {
        assert!(Profile::from_icc(b"not an icc profile").is_err());
    }
}
