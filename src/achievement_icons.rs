use anyhow::{Context, Result, ensure};
use image::{ImageFormat, ImageReader};
use serde_json::{Value, json};
use std::{io::Cursor, path::Path};

const SOURCE: &str = "https://partner.steamgames.com/doc/features/achievements/ach_guide#icons";

/// Check actual encoded contents, rather than trusting a file extension.
pub fn validate(path: &Path) -> Result<Value> {
    let bytes = std::fs::read(path).context("Cannot read achievement icon")?;
    validate_bytes(&bytes)
}

fn validate_bytes(bytes: &[u8]) -> Result<Value> {
    let format = image::guess_format(bytes).context("Unrecognized achievement icon format")?;
    ensure!(
        matches!(format, ImageFormat::Png | ImageFormat::Jpeg),
        "Achievement icons must be PNG or JPEG"
    );
    // Default decoder limits bound allocations; decoding also rejects damaged files.
    let decoded = ImageReader::with_format(Cursor::new(bytes), format)
        .decode()
        .context("Cannot decode achievement icon")?;
    let (width, height) = (decoded.width(), decoded.height());
    ensure!(
        width == height,
        "Achievement icons must be square; got {width}x{height}"
    );
    ensure!(
        width >= 64,
        "Achievement icons must be at least 64x64; got {width}x{height}"
    );
    let mut recommendations = Vec::new();
    if width != 256 {
        recommendations.push("Steam recommends 256x256; larger icons are scaled down by Steam.");
    }
    if format != ImageFormat::Png {
        recommendations.push("Steam recommends PNG; JPEG is also accepted.");
    }
    Ok(json!({
        "valid":true,"format":if format == ImageFormat::Png {"PNG"} else {"JPEG"},
        "width":width,"height":height,"recommendations":recommendations,
        "source":SOURCE,"uploaded":false
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn encoded(width: u32, height: u32, format: ImageFormat) -> Vec<u8> {
        let mut out = Cursor::new(Vec::new());
        image::DynamicImage::new_rgb8(width, height)
            .write_to(&mut out, format)
            .unwrap();
        out.into_inner()
    }

    #[test]
    fn accepts_documented_formats_and_sizes() {
        for format in [ImageFormat::Png, ImageFormat::Jpeg] {
            for size in [64, 256, 512] {
                let report = validate_bytes(&encoded(size, size, format)).unwrap();
                assert_eq!(report["width"], size);
                assert_eq!(report["valid"], true);
            }
        }
    }

    #[test]
    fn rejects_wrong_shape_small_or_corrupted_images() {
        assert!(validate_bytes(&encoded(256, 128, ImageFormat::Png)).is_err());
        assert!(validate_bytes(&encoded(32, 32, ImageFormat::Png)).is_err());
        let mut broken = encoded(256, 256, ImageFormat::Png);
        broken.truncate(broken.len() / 2);
        assert!(validate_bytes(&broken).is_err());
        assert!(validate_bytes(b"not an image").is_err());
        assert!(validate_bytes(b"GIF89a\x01\x00\x01\x00").is_err());
    }
}
