//! The OpenSeries icons, drawn in code (no image assets; clean room, AGENTS.md).
//!
//! * [`headphone_rgba`]: the plain headphone glyph for the tray / menu bar.
//! * [`app_icon_rgba`]: the app icon (white headphone on an accent-coloured rounded square) for
//!   the window, the `.exe`, the installer and the macOS bundle. The bundled files in
//!   `crates/ss-app/bundle/icon/` are generated from it; a test keeps them in sync
//!   (`OPENSERIES_WRITE_ICONS=1 cargo test -p ss-app icon` rewrites them).

/// Accent colour of the app icon background (`ui-design.md` §3, dark accent).
pub const ACCENT: [u8; 3] = [0x5B, 0x7C, 0xFA];
/// Glyph colour on the app icon.
const GLYPH: [u8; 3] = [0xFF, 0xFF, 0xFF];
/// Empty border around the rounded square, as a share of the edge (macOS icon grid ≈ 10 %).
const ICON_MARGIN: f32 = 0.08;
/// Corner radius of the rounded square, as a share of its edge.
const ICON_RADIUS: f32 = 0.225;
/// Size of the glyph relative to the rounded square.
const GLYPH_SCALE: f32 = 0.7;
/// Supersampling grid per pixel edge for smooth edges.
const SUPERSAMPLING: u32 = 4;

/// Whether the point (`u`, `v`) of the unit square lies on the headphone glyph.
fn on_headphone(u: f32, v: f32) -> bool {
    let (centre, outer, inner) = (0.5, 0.42, 0.32);
    let distance = ((u - centre).powi(2) + (v - centre).powi(2)).sqrt();
    let headband = v <= centre && distance <= outer && distance >= inner;
    let cups = v >= centre - 0.05
        && v <= 0.9
        && ((u - (centre - outer)).abs() <= 0.09 || (u - (centre + outer)).abs() <= 0.09);
    headband || cups
}

/// Whether (`u`, `v`) lies inside the rectangle `(left, top)–(right, bottom)` with corner
/// radius `radius`.
fn in_rounded_rect(
    u: f32,
    v: f32,
    (left, top, right, bottom): (f32, f32, f32, f32),
    radius: f32,
) -> bool {
    let cx = u.clamp(left + radius, right - radius);
    let cy = v.clamp(top + radius, bottom - radius);
    u >= left
        && u <= right
        && v >= top
        && v <= bottom
        && (u - cx).powi(2) + (v - cy).powi(2) <= radius * radius
}

/// Whether (`u`, `v`) lies inside the rounded square of the app icon.
fn on_background(u: f32, v: f32) -> bool {
    let (low, high) = (ICON_MARGIN, 1.0 - ICON_MARGIN);
    in_rounded_rect(u, v, (low, low, high, high), ICON_RADIUS * (high - low))
}

/// The smoother headphone of the app icon (unit square): a headband arc that runs straight down
/// into two rounded ear cups.
fn on_app_glyph(u: f32, v: f32) -> bool {
    const CENTRE: f32 = 0.5;
    const ARC_CENTRE_V: f32 = 0.52;
    const OUTER: f32 = 0.40;
    const INNER: f32 = 0.31;
    const LEGS_END: f32 = 0.66;
    const CUP_CENTRE: f32 = 0.355;
    const CUP_HALF_WIDTH: f32 = 0.105;
    const CUP_TOP: f32 = 0.52;
    const CUP_BOTTOM: f32 = 0.9;
    const CUP_RADIUS: f32 = 0.08;
    let dx = u - CENTRE;
    let distance = (dx.powi(2) + (v - ARC_CENTRE_V).powi(2)).sqrt();
    let arc = v <= ARC_CENTRE_V && (INNER..=OUTER).contains(&distance);
    let legs = v > ARC_CENTRE_V && v <= LEGS_END && (INNER..=OUTER).contains(&dx.abs());
    let cup = |centre: f32| {
        in_rounded_rect(
            u,
            v,
            (
                centre - CUP_HALF_WIDTH,
                CUP_TOP,
                centre + CUP_HALF_WIDTH,
                CUP_BOTTOM,
            ),
            CUP_RADIUS,
        )
    };
    arc || legs || cup(CENTRE - CUP_CENTRE) || cup(CENTRE + CUP_CENTRE)
}

/// The headphone glyph (RGBA, `size`×`size`, transparent background) in `color`, one sample per
/// pixel so small tray icons stay crisp.
#[must_use]
pub fn headphone_rgba(size: u32, color: [u8; 3]) -> Vec<u8> {
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    #[allow(clippy::cast_precision_loss)] // small icon sizes
    let extent = size as f32;
    for y in 0..size {
        for x in 0..size {
            #[allow(clippy::cast_precision_loss)]
            let (u, v) = ((x as f32 + 0.5) / extent, (y as f32 + 0.5) / extent);
            if on_headphone(u, v) {
                let index = ((y * size + x) * 4) as usize;
                rgba[index..index + 3].copy_from_slice(&color);
                rgba[index + 3] = 255;
            }
        }
    }
    rgba
}

/// The app icon (RGBA, `size`×`size`), anti-aliased.
#[must_use]
pub fn app_icon_rgba(size: u32) -> Vec<u8> {
    let mut rgba = vec![0_u8; (size * size * 4) as usize];
    #[allow(clippy::cast_precision_loss)] // icon sizes up to 1024
    let (extent, grid) = (size as f32, SUPERSAMPLING as f32);
    let samples = grid * grid;
    let glyph_offset = (1.0 - GLYPH_SCALE) / 2.0;
    for y in 0..size {
        for x in 0..size {
            let (mut background, mut glyph) = (0.0_f32, 0.0_f32);
            for sy in 0..SUPERSAMPLING {
                for sx in 0..SUPERSAMPLING {
                    #[allow(clippy::cast_precision_loss)]
                    let u = (x as f32 + (sx as f32 + 0.5) / grid) / extent;
                    #[allow(clippy::cast_precision_loss)]
                    let v = (y as f32 + (sy as f32 + 0.5) / grid) / extent;
                    if on_background(u, v) {
                        background += 1.0;
                        let (gu, gv) = (
                            (u - glyph_offset) / GLYPH_SCALE,
                            (v - glyph_offset) / GLYPH_SCALE,
                        );
                        if on_app_glyph(gu, gv) {
                            glyph += 1.0;
                        }
                    }
                }
            }
            if background == 0.0 {
                continue;
            }
            // Colour: glyph share within the covered part; alpha: covered share.
            let mix = glyph / background;
            let index = ((y * size + x) * 4) as usize;
            for channel in 0..3 {
                let value = f32::from(ACCENT[channel])
                    + (f32::from(GLYPH[channel]) - f32::from(ACCENT[channel])) * mix;
                rgba[index + channel] = to_byte(value);
            }
            rgba[index + 3] = to_byte(255.0 * background / samples);
        }
    }
    rgba
}

#[allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)] // clamped to 0..=255
fn to_byte(value: f32) -> u8 {
    value.round().clamp(0.0, 255.0) as u8
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::{Path, PathBuf};

    /// Sizes inside the Windows `.ico` (Explorer, taskbar, Start menu, installer).
    const ICO_SIZES: [u32; 7] = [16, 24, 32, 48, 64, 128, 256];
    /// Edge of the macOS master image (`app.sh` derives the `.icns` from it).
    const MASTER_SIZE: u32 = 1024;

    fn png(size: u32) -> Vec<u8> {
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, size, size);
        encoder.set_color(png::ColorType::Rgba);
        encoder.set_depth(png::BitDepth::Eight);
        encoder.set_compression(png::Compression::Balanced);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&app_icon_rgba(size)).unwrap();
        writer.finish().unwrap();
        bytes
    }

    /// An `.ico` with PNG-compressed entries (supported since Windows Vista).
    fn ico() -> Vec<u8> {
        let images: Vec<Vec<u8>> = ICO_SIZES.iter().map(|&s| png(s)).collect();
        let count = u16::try_from(ICO_SIZES.len()).unwrap();
        let mut bytes = Vec::new();
        bytes.extend_from_slice(&[0, 0, 1, 0]); // reserved, type 1 = icon
        bytes.extend_from_slice(&count.to_le_bytes());
        let mut offset = 6 + 16 * u32::from(count);
        for (&size, image) in ICO_SIZES.iter().zip(&images) {
            let edge = u8::try_from(size % 256).unwrap(); // 0 means 256
            bytes.extend_from_slice(&[edge, edge, 0, 0]);
            bytes.extend_from_slice(&1_u16.to_le_bytes()); // colour planes
            bytes.extend_from_slice(&32_u16.to_le_bytes()); // bits per pixel
            let length = u32::try_from(image.len()).unwrap();
            bytes.extend_from_slice(&length.to_le_bytes());
            bytes.extend_from_slice(&offset.to_le_bytes());
            offset += length;
        }
        for image in &images {
            bytes.extend_from_slice(image);
        }
        bytes
    }

    fn icon_dir() -> PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("bundle/icon")
    }

    #[test]
    fn bundled_icons_are_up_to_date() {
        let files = [("OpenSeries.ico", ico()), ("AppIcon.png", png(MASTER_SIZE))];
        let write = std::env::var_os("OPENSERIES_WRITE_ICONS").is_some();
        for (name, expected) in files {
            let path = icon_dir().join(name);
            if write {
                std::fs::create_dir_all(icon_dir()).unwrap();
                std::fs::write(&path, &expected).unwrap();
            }
            let actual = std::fs::read(&path).unwrap_or_default();
            assert!(
                actual == expected,
                "{name} is out of date: OPENSERIES_WRITE_ICONS=1 cargo test -p ss-app icon"
            );
        }
    }

    #[test]
    fn app_icon_has_transparent_corners_and_a_white_glyph() {
        let size = 64;
        let rgba = app_icon_rgba(size);
        assert_eq!(rgba[3], 0, "top-left corner is transparent");
        let at = |x: u32, y: u32| &rgba[((y * size + x) * 4) as usize..][..4];
        // Middle of the left ear cup: glyph colour, opaque.
        assert_eq!(at(14, 44), &[0xFF, 0xFF, 0xFF, 0xFF]);
        // Centre (inside the headband arc, below it): background colour.
        assert_eq!(at(32, 26)[..3], ACCENT);
    }
}
