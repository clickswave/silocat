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

/// Most pixels a display picture may have (16 MP, 4096 x 4096). Decode and
/// resize time grow with the pixel count, and a flat-colour PNG this size is
/// only kilobytes, so the byte cap alone does not bound the work.
pub const MAX_DP_PIXELS: u64 = 4096 * 4096;

/// The image is valid but bigger than a display picture may be.
#[derive(Debug)]
pub struct TooLarge;

impl std::fmt::Display for TooLarge {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Image is larger than {MAX_DP_PIXELS} pixels")
    }
}

impl std::error::Error for TooLarge {}

/// Decode -> center square crop -> resize to EDGE x EDGE -> JPEG bytes.
/// CPU-bound for up to a few hundred milliseconds: call it off the async
/// runtime.
pub fn normalize(input: &[u8]) -> anyhow::Result<Vec<u8>> {
    // Bound the decode so a small but adversarial file (a "decompression bomb")
    // can't expand to gigabytes of pixels and OOM the process. The byte size is
    // already capped upstream (MAX_DP_BYTES); this caps the *decoded* image.
    let mut reader = image::ImageReader::new(std::io::Cursor::new(input))
        .with_guessed_format()
        .map_err(|e| anyhow::anyhow!("Unsupported or corrupt image: {e}"))?;
    let mut limits = image::Limits::default();
    limits.max_image_width = Some(8192);
    limits.max_image_height = Some(8192);
    limits.max_alloc = Some(64 * 1024 * 1024); // 64 MB decode budget
    reader.limits(limits.clone());
    let refuse = |e: image::ImageError| match e {
        image::ImageError::Limits(_) => anyhow::Error::new(TooLarge),
        e => anyhow::anyhow!("Unsupported or corrupt image: {e}"),
    };
    let mut decoder = reader.into_decoder().map_err(refuse)?;
    // The header gives the size, so an oversized image is refused before any
    // pixel is decoded.
    let (w, h) = image::ImageDecoder::dimensions(&decoder);
    if u64::from(w) * u64::from(h) > MAX_DP_PIXELS {
        return Err(anyhow::Error::new(TooLarge));
    }
    // into_decoder() only checks the dimensions, so charge the decoded buffer
    // to the 64 MB budget here, as ImageReader::decode() would. A 16-bit RGBA
    // image at the pixel cap needs 128 MB and is refused.
    limits.reserve(image::ImageDecoder::total_bytes(&decoder)).map_err(refuse)?;
    image::ImageDecoder::set_limits(&mut decoder, limits).map_err(refuse)?;
    let img = image::DynamicImage::from_decoder(decoder).map_err(refuse)?;

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

/// An all-black grayscale PNG, for tests that need a valid image of a given size.
#[cfg(test)]
pub fn test_png(width: u32, height: u32) -> Vec<u8> {
    use image::ImageEncoder;
    let pixels = vec![0u8; width as usize * height as usize];
    let mut out = Vec::new();
    image::codecs::png::PngEncoder::new_with_quality(
        &mut out,
        image::codecs::png::CompressionType::Fast,
        image::codecs::png::FilterType::NoFilter,
    )
    .write_image(&pixels, width, height, image::ExtendedColorType::L8)
    .expect("encode test png");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_small_file_cannot_ask_for_more_than_sixteen_megapixels() {
        // 8192 x 2049 is inside the per-edge limit and a few kilobytes on the
        // wire, but decoding and resizing it costs as much as a 16.8 MP photo.
        let bytes = test_png(8192, 2049);
        assert!(bytes.len() < MAX_DP_BYTES);
        for (w, h, bytes) in [(8192, 2049, bytes), (4097, 4097, test_png(4097, 4097)), (8193, 1, test_png(8193, 1))] {
            match normalize(&bytes) {
                Ok(_) => panic!("{w} x {h} was accepted"),
                Err(e) => assert!(e.is::<TooLarge>(), "{w} x {h}: {e}"),
            }
        }
    }

    #[test]
    fn a_deep_colour_file_cannot_decode_past_the_memory_budget() {
        // 4096 x 4096 is within the pixel cap, but at 16 bits per channel the
        // decoded buffer is 128 MiB (RGBA) or 96 MiB (RGB), over the 64 MiB
        // decode budget, while the flat PNG is still small on the wire.
        for (color, bpp) in [(image::ExtendedColorType::Rgba16, 8), (image::ExtendedColorType::Rgb16, 6)] {
            use image::ImageEncoder;
            let pixels = vec![0u8; 4096 * 4096 * bpp];
            let mut bytes = Vec::new();
            image::codecs::png::PngEncoder::new_with_quality(
                &mut bytes,
                image::codecs::png::CompressionType::Fast,
                image::codecs::png::FilterType::NoFilter,
            )
            .write_image(&pixels, 4096, 4096, color)
            .expect("encode test png");
            drop(pixels);
            assert!(bytes.len() < MAX_DP_BYTES, "{color:?}: {} bytes", bytes.len());
            match normalize(&bytes) {
                Ok(_) => panic!("{color:?} at 4096 x 4096 was accepted"),
                Err(e) => assert!(e.is::<TooLarge>(), "{color:?}: {e}"),
            }
        }
    }

    #[test]
    fn ordinary_shapes_still_normalize() {
        for (w, h) in [(4096, 8), (8, 4096), (640, 480), (1, 1)] {
            let out = normalize(&test_png(w, h)).unwrap_or_else(|e| panic!("{w} x {h}: {e}"));
            let img = image::load_from_memory(&out).expect("output decodes");
            assert_eq!((img.width(), img.height()), (EDGE, EDGE));
        }
    }
}
