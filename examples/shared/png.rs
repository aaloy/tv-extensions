// (C) 2026 - Enzo Lombardi

//! Lossless recompression of the PNGs `render_to_png` writes, shared by the
//! examples that save pictures (`make_logo`, and the gallery's captures).

use std::io::{Read, Write};
use std::path::Path;

use flate2::Compression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;

/// The 8-byte PNG file signature.
const PNG_SIGNATURE: [u8; 8] = [0x89, 0x50, 0x4e, 0x47, 0x0d, 0x0a, 0x1a, 0x0a];

/// Re-deflate a PNG written by `render_to_png` (which uses only stored,
/// non-compressed DEFLATE blocks) at the best compression level, in place.
/// This only touches how the pixels are packed, never the pixels
/// themselves: it decompresses the existing `IDAT` stream, recompresses
/// the identical bytes, and rewrites the chunk (and its CRC).
pub fn recompress_png(path: &Path) -> std::io::Result<()> {
    let bytes = std::fs::read(path)?;
    assert_eq!(&bytes[..8], &PNG_SIGNATURE, "not a PNG file");

    let mut ihdr = None;
    let mut idat = Vec::new();
    let mut pos = 8;
    while pos + 8 <= bytes.len() {
        let len = u32::from_be_bytes(bytes[pos..pos + 4].try_into().unwrap()) as usize;
        let kind = &bytes[pos + 4..pos + 8];
        let data = &bytes[pos + 8..pos + 8 + len];
        match kind {
            b"IHDR" => ihdr = Some(data.to_vec()),
            b"IDAT" => idat.extend_from_slice(data),
            _ => {}
        }
        pos += 8 + len + 4; // chunk length + type + data + crc
    }
    let ihdr = ihdr.expect("render_to_png always writes an IHDR chunk");

    let mut raw = Vec::new();
    ZlibDecoder::new(&idat[..]).read_to_end(&mut raw)?;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::best());
    encoder.write_all(&raw)?;
    let compressed = encoder.finish()?;

    let mut out = Vec::with_capacity(PNG_SIGNATURE.len() + compressed.len() + 64);
    out.extend_from_slice(&PNG_SIGNATURE);
    write_chunk(&mut out, *b"IHDR", &ihdr);
    write_chunk(&mut out, *b"IDAT", &compressed);
    write_chunk(&mut out, *b"IEND", &[]);
    std::fs::write(path, out)
}

/// Append one PNG chunk (length, type, data, CRC-32) to `out`. `data` is
/// always a handful of kilobytes here, far under `u32::MAX`.
#[allow(
    clippy::cast_possible_truncation,
    reason = "A PNG chunk this example writes is never close to 4 GiB."
)]
fn write_chunk(out: &mut Vec<u8>, kind: [u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(&kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(&kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// CRC-32 (IEEE 802.3 polynomial), the checksum PNG chunks use.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &b in bytes {
        crc ^= u32::from(b);
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}
