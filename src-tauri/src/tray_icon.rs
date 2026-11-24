//! Tray icons in the markpitt.dev brand, drawn at runtime so no image files are
//! needed per state: the plain disc (an orange square with a charcoal circle),
//! with the overall state as a small dot in the corner. The tooltip gives the word.

use crate::status::Level;
use tauri::image::Image;

pub const SIZE: u32 = 64;

/// `--accent-mark`, the square.
const MARK: [u8; 3] = [0xce, 0x4b, 0x00];
/// `--charcoal`, the circle and the ring round the status dot.
const CHARCOAL: [u8; 3] = [0x2e, 0x28, 0x23];

/// The status dot: `--success`, `--warning`, `--danger`, or `--muted` for unknown (dark theme values,
/// which read on both light and dark trays inside the charcoal ring).
fn dot(level: Level) -> [u8; 3] {
    match level {
        Level::Ok => [0x4a, 0xde, 0x80],
        Level::Warn => [0xfa, 0xcc, 0x15],
        Level::Bad => [0xff, 0x8a, 0x8a],
        Level::Unknown => [0x9c, 0x8c, 0x7d],
    }
}

/// One-pixel soft edge: how much of a pixel at distance `d` lies inside radius `r`.
fn cover(r: f32, d: f32) -> f32 {
    (r + 0.5 - d).clamp(0.0, 1.0)
}

fn mix(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    let m = |x: u8, y: u8| (x as f32 + (y as f32 - x as f32) * t).round() as u8;
    [m(a[0], b[0]), m(a[1], b[1]), m(a[2], b[2])]
}

/// RGBA pixels for a level's icon. Separate from `icon` so it can be tested.
pub fn rgba(level: Level) -> Vec<u8> {
    let size = SIZE as f32;
    let centre = size / 2.0;
    // 145 of 460 in the brand's SVG.
    let disc_r = size * 145.0 / 460.0;
    let (dot_cx, dot_cy) = (size - 13.0, size - 13.0);
    let dot_r = 10.0;
    let ring = 3.0;
    let fill = dot(level);
    let mut px = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let (fx, fy) = (x as f32 + 0.5, y as f32 + 0.5);
            let d = ((fx - centre).powi(2) + (fy - centre).powi(2)).sqrt();
            let mut c = mix(MARK, CHARCOAL, cover(disc_r, d));
            let dd = ((fx - dot_cx).powi(2) + (fy - dot_cy).powi(2)).sqrt();
            c = mix(c, CHARCOAL, cover(dot_r + ring, dd));
            c = mix(c, fill, cover(dot_r, dd));
            px.extend_from_slice(&[c[0], c[1], c[2], 255]);
        }
    }
    px
}

pub fn icon(level: Level) -> Image<'static> {
    Image::new_owned(rgba(level), SIZE, SIZE)
}

/// The state in one word, for the tooltip.
pub fn word(level: Level) -> &'static str {
    match level {
        Level::Ok => "Operational",
        Level::Warn => "Degraded",
        Level::Bad => "Outage",
        Level::Unknown => "Unknown",
    }
}

/// Tooltip text for the tray: the word, then the feed's sentence.
pub fn tooltip(level: Level, description: &str) -> String {
    format!("Claude Overview: {}. {description}", word(level))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(px: &[u8], x: u32, y: u32) -> &[u8] {
        let i = ((y * SIZE + x) * 4) as usize;
        &px[i..i + 4]
    }

    #[test]
    fn disc_is_an_orange_square_with_a_charcoal_centre() {
        let px = rgba(Level::Ok);
        assert_eq!(px.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(at(&px, 0, 0), &[0xce, 0x4b, 0x00, 255], "corner is the orange square");
        assert_eq!(at(&px, SIZE / 2, SIZE / 2), &[0x2e, 0x28, 0x23, 255], "centre is charcoal");
    }

    #[test]
    fn the_corner_dot_carries_the_state() {
        let (x, y) = (SIZE - 13, SIZE - 13);
        assert_eq!(&at(&rgba(Level::Ok), x, y)[..3], &[0x4a, 0xde, 0x80]);
        assert_eq!(&at(&rgba(Level::Bad), x, y)[..3], &[0xff, 0x8a, 0x8a]);
        assert_ne!(at(&rgba(Level::Warn), x, y), at(&rgba(Level::Unknown), x, y));
    }

    #[test]
    fn tooltip_gives_the_word() {
        assert_eq!(tooltip(Level::Warn, "Partially degraded service"), "Claude Overview: Degraded. Partially degraded service");
    }
}
