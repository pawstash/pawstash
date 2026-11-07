use image::ImageReader;
use serde::Serialize;
use std::io::Cursor;

#[derive(Clone, Debug, Serialize)]
pub struct MediaInfo {
    pub width: u32,
    pub height: u32,
    pub format: Option<String>,
}

const MP4_FIXED_POINT: f64 = 65536.0;

pub fn from_bytes(bytes: &[u8]) -> Option<MediaInfo> {
    if let Some(info) = image_info(bytes) {
        return Some(info);
    }
    mp4_info(bytes)
}

pub fn from_mp4_tail(bytes: &[u8]) -> Option<MediaInfo> {
    let marker = bytes.windows(4).position(|w| w == b"moov")?;
    let start = marker.checked_sub(4)?;
    let size = read_u32(bytes, start)? as usize;
    let end = start.checked_add(size)?.min(bytes.len());
    if size < 8 || end <= marker + 4 {
        return None;
    }
    tracks_dimensions(&bytes[marker + 4..end])
}

fn image_info(bytes: &[u8]) -> Option<MediaInfo> {
    let reader = ImageReader::new(Cursor::new(bytes))
        .with_guessed_format()
        .ok()?;
    let format = reader.format().map(|format| format!("{format:?}"));
    let (width, height) = reader.into_dimensions().ok()?;
    (width > 0 && height > 0).then_some(MediaInfo {
        width,
        height,
        format,
    })
}

fn mp4_info(bytes: &[u8]) -> Option<MediaInfo> {
    if bytes.get(4..8) != Some(b"ftyp") {
        return None;
    }
    let moov = find_box(bytes, 0, bytes.len(), b"moov")?;
    tracks_dimensions(&bytes[moov.payload..moov.end])
}

fn tracks_dimensions(moov: &[u8]) -> Option<MediaInfo> {
    let mut position = 0;
    while let Some(trak) = find_box(moov, position, moov.len(), b"trak") {
        let payload = &moov[trak.payload..trak.end];
        if let Some(tkhd) = find_box(payload, 0, payload.len(), b"tkhd") {
            if let Some((width, height)) = tkhd_dimensions(payload, &tkhd) {
                return Some(MediaInfo {
                    width,
                    height,
                    format: Some("mp4".to_string()),
                });
            }
        }
        position = trak.end;
        if position <= trak.start {
            break;
        }
    }
    None
}

fn tkhd_dimensions(data: &[u8], header: &BoxHeader) -> Option<(u32, u32)> {
    let version = *data.get(header.payload)?;
    let (width_at, height_at) = if version == 1 {
        (header.start + 96, header.start + 100)
    } else {
        (header.start + 84, header.start + 88)
    };
    let width = read_u32(data, width_at)? as f64 / MP4_FIXED_POINT;
    let height = read_u32(data, height_at)? as f64 / MP4_FIXED_POINT;
    let width = width.round() as u32;
    let height = height.round() as u32;
    (width > 0 && height > 0).then_some((width, height))
}

#[derive(Clone, Copy)]
struct BoxHeader {
    start: usize,
    payload: usize,
    end: usize,
}

fn find_box(data: &[u8], start: usize, end: usize, kind: &[u8; 4]) -> Option<BoxHeader> {
    let mut position = start;
    while position + 8 <= end {
        let size = read_u32(data, position)? as usize;
        let box_kind = data.get(position + 4..position + 8)?;
        let (header_len, size) = if size == 1 {
            (16usize, read_u64(data, position + 8)? as usize)
        } else if size == 0 {
            (8usize, end - position)
        } else {
            (8usize, size as usize)
        };
        if size < header_len || position + size > end {
            return None;
        }
        let header = BoxHeader {
            start: position,
            payload: position + header_len,
            end: position + size,
        };
        if box_kind == kind {
            return Some(header);
        }
        position = header.end;
    }
    None
}

fn read_u32(data: &[u8], at: usize) -> Option<u32> {
    let bytes = data.get(at..at + 4)?;
    Some(u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]))
}

fn read_u64(data: &[u8], at: usize) -> Option<u64> {
    let bytes = data.get(at..at + 8)?;
    Some(u64::from_be_bytes([
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn png(width: u32, height: u32) -> Vec<u8> {
        let image = image::RgbaImage::from_pixel(width, height, image::Rgba([1, 2, 3, 255]));
        let mut bytes = Vec::new();
        image
            .write_to(&mut Cursor::new(&mut bytes), image::ImageFormat::Png)
            .unwrap();
        bytes
    }

    fn mp4(width: u32, height: u32) -> Vec<u8> {
        let mut tkhd = Vec::new();
        tkhd.extend_from_slice(&0u32.to_be_bytes());
        tkhd.extend_from_slice(b"tkhd");
        tkhd.extend_from_slice(&[0, 0, 0, 0]);
        tkhd.extend_from_slice(&[0; 72]);
        tkhd.extend_from_slice(&(width << 16).to_be_bytes());
        tkhd.extend_from_slice(&(height << 16).to_be_bytes());
        let size = tkhd.len() as u32;
        tkhd[0..4].copy_from_slice(&size.to_be_bytes());

        let mut trak = Vec::new();
        trak.extend_from_slice(&((tkhd.len() + 8) as u32).to_be_bytes());
        trak.extend_from_slice(b"trak");
        trak.extend_from_slice(&tkhd);

        let mut moov = Vec::new();
        moov.extend_from_slice(&((trak.len() + 8) as u32).to_be_bytes());
        moov.extend_from_slice(b"moov");
        moov.extend_from_slice(&trak);

        let mut ftyp = Vec::new();
        ftyp.extend_from_slice(&20u32.to_be_bytes());
        ftyp.extend_from_slice(b"ftyp");
        ftyp.extend_from_slice(b"isom");
        ftyp.extend_from_slice(&[0; 8]);
        ftyp.extend_from_slice(&moov);
        ftyp
    }

    #[test]
    fn reads_png_dimensions() {
        let info = from_bytes(&png(7, 5)).unwrap();
        assert_eq!((info.width, info.height), (7, 5));
    }

    #[test]
    fn reads_mp4_tkhd_dimensions() {
        let info = from_bytes(&mp4(1920, 1080)).unwrap();
        assert_eq!((info.width, info.height), (1920, 1080));
    }

    #[test]
    fn reads_moov_from_tail() {
        let data = mp4(1280, 720);
        let tail = &data[8..];
        let info = from_mp4_tail(tail).unwrap();
        assert_eq!((info.width, info.height), (1280, 720));
    }
}
