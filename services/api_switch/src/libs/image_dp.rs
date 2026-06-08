//! Display-picture normalization.
//!
//! Whatever the user uploads (JPEG/PNG/WebP/GIF/BMP, any size/aspect), we decode
//! it, center-crop to the largest square, resize to a fixed edge, and re-encode
//! as JPEG. Every avatar then has identical dimensions and format, and the
//! re-encode strips EXIF (orientation, GPS, etc.) as a side effect.

use image::{imageops::FilterType, GenericImageView};

/// Maximum accepted upload size for a display picture (1 MB).
pub const MAX_DP_BYTES: usize = 1024 * 1024;

/// Output square edge, in pixels.
const EDGE: u32 = 512;

/// JPEG quality (1-100). 85 is a good size/quality tradeoff for avatars.
const JPEG_QUALITY: u8 = 85;

/// Decode -> center square crop -> resize to EDGE x EDGE -> JPEG bytes.
pub fn normalize(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    let img = image::load_from_memory(input)
        .map_err(|e| anyhow::anyhow!("Unsupported or corrupt image: {e}"))?;

    let (w, h) = img.dimensions();
    if w == 0 || h == 0 {
        return Err(anyhow::anyhow!("Image has a zero dimension"));
    }

    // Largest centered square.
    let edge = w.min(h);
    let x = (w - edge) / 2;
    let y = (h - edge) / 2;
    let square = img.crop_imm(x, y, edge, edge);

    // Fixed square output.
    let resized = square.resize_exact(EDGE, EDGE, FilterType::Lanczos3);

    // JPEG has no alpha, so flatten to RGB before encoding.
    let rgb = resized.to_rgb8();
    let mut out = Vec::new();
    let mut encoder = image::codecs::jpeg::JpegEncoder::new_with_quality(&mut out, JPEG_QUALITY);
    encoder.encode(rgb.as_raw(), EDGE, EDGE, image::ExtendedColorType::Rgb8)?;
    Ok(out)
}
