pub struct ImageInfo {
    pub content_type: &'static str,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ImageError {
    Invalid,
    TooManyPixels,
}

impl std::fmt::Display for ImageError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid => f.write_str("unsupported or malformed image"),
            Self::TooManyPixels => f.write_str("image dimensions exceed configured pixel limit"),
        }
    }
}

impl std::error::Error for ImageError {}

pub fn inspect_image(bytes: &[u8], max_pixels: u64) -> Result<ImageInfo, ImageError> {
    let info = if bytes.len() >= 24 && bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10] {
        if &bytes[12..16] != b"IHDR" {
            return Err(ImageError::Invalid);
        }
        let width = u32::from_be_bytes(bytes[16..20].try_into().map_err(|_| ImageError::Invalid)?);
        let height = u32::from_be_bytes(bytes[20..24].try_into().map_err(|_| ImageError::Invalid)?);
        ImageInfo {
            content_type: "image/png",
            width,
            height,
        }
    } else if bytes.len() >= 4 && bytes[0] == 0xff && bytes[1] == 0xd8 {
        inspect_jpeg(bytes)?
    } else {
        return Err(ImageError::Invalid);
    };
    if info.width == 0 || info.height == 0 {
        return Err(ImageError::Invalid);
    }
    let pixels = u64::from(info.width)
        .checked_mul(u64::from(info.height))
        .ok_or(ImageError::TooManyPixels)?;
    if pixels > max_pixels {
        return Err(ImageError::TooManyPixels);
    }
    Ok(info)
}

fn inspect_jpeg(bytes: &[u8]) -> Result<ImageInfo, ImageError> {
    let mut i = 2usize;
    while i + 4 <= bytes.len() {
        if bytes[i] != 0xff {
            i += 1;
            continue;
        }
        while i < bytes.len() && bytes[i] == 0xff {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let marker = bytes[i];
        i += 1;
        if matches!(marker, 0xd8 | 0xd9) {
            continue;
        }
        if marker == 0xda {
            break;
        }
        if i + 2 > bytes.len() {
            return Err(ImageError::Invalid);
        }
        let len = u16::from_be_bytes([bytes[i], bytes[i + 1]]) as usize;
        if len < 2 || i + len > bytes.len() {
            return Err(ImageError::Invalid);
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) {
            if len < 7 {
                return Err(ImageError::Invalid);
            }
            let height = u16::from_be_bytes([bytes[i + 3], bytes[i + 4]]) as u32;
            let width = u16::from_be_bytes([bytes[i + 5], bytes[i + 6]]) as u32;
            return Ok(ImageInfo {
                content_type: "image/jpeg",
                width,
                height,
            });
        }
        i += len;
    }
    Err(ImageError::Invalid)
}

#[cfg(test)]
mod image_tests {
    use super::{ImageError, inspect_image};
    #[test]
    fn png_magic_and_dimensions_are_authoritative() {
        let mut b = vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        b.extend_from_slice(&320u32.to_be_bytes());
        b.extend_from_slice(&200u32.to_be_bytes());
        b.extend_from_slice(&[8, 2, 0, 0, 0]);
        let i = inspect_image(&b, 1_000_000).unwrap();
        assert_eq!(i.content_type, "image/png");
        assert_eq!((i.width, i.height), (320, 200));
    }
    #[test]
    fn svg_and_pixel_bombs_are_rejected() {
        assert!(matches!(
            inspect_image(b"<svg onload='x'>", 1_000_000),
            Err(ImageError::Invalid)
        ));
        let mut b = vec![
            137, 80, 78, 71, 13, 10, 26, 10, 0, 0, 0, 13, b'I', b'H', b'D', b'R',
        ];
        b.extend_from_slice(&50_000u32.to_be_bytes());
        b.extend_from_slice(&50_000u32.to_be_bytes());
        b.extend_from_slice(&[8, 2, 0, 0, 0]);
        assert!(matches!(
            inspect_image(&b, 40_000_000),
            Err(ImageError::TooManyPixels)
        ));
    }
}
