//! Status-coloured tray icons, drawn at runtime so no image files are needed
//! per state. Each icon is an anti-aliased disc with a darker rim.

use crate::status::Level;
use tauri::image::Image;

pub const SIZE: u32 = 64;

/// (fill, rim) colours per level, as RGB.
fn colours(level: Level) -> ([u8; 3], [u8; 3]) {
    match level {
        Level::Ok => ([0x2e, 0xa0, 0x5f], [0x1d, 0x6b, 0x3f]),
        Level::Warn => ([0xe0, 0x9a, 0x2b], [0x9a, 0x66, 0x14]),
        Level::Bad => ([0xd6, 0x45, 0x45], [0x8f, 0x26, 0x26]),
        Level::Unknown => ([0x8a, 0x8a, 0x8a], [0x5c, 0x5c, 0x5c]),
    }
}

/// RGBA pixels for a level's icon. Separate from `icon` so it can be tested.
pub fn rgba(level: Level) -> Vec<u8> {
    let (fill, rim) = colours(level);
    let size = SIZE as f32;
    let centre = size / 2.0;
    let radius = size / 2.0 - 4.0;
    let rim_width = 5.0;
    let mut px = Vec::with_capacity((SIZE * SIZE * 4) as usize);
    for y in 0..SIZE {
        for x in 0..SIZE {
            let dx = x as f32 + 0.5 - centre;
            let dy = y as f32 + 0.5 - centre;
            let d = (dx * dx + dy * dy).sqrt();
            // Coverage for a one-pixel soft edge.
            let alpha = (radius + 0.5 - d).clamp(0.0, 1.0);
            let c = if d > radius - rim_width { rim } else { fill };
            px.extend_from_slice(&[c[0], c[1], c[2], (alpha * 255.0) as u8]);
        }
    }
    px
}

pub fn icon(level: Level) -> Image<'static> {
    Image::new_owned(rgba(level), SIZE, SIZE)
}

/// Tooltip text for the tray.
pub fn tooltip(level: Level, description: &str) -> String {
    let prefix = match level {
        Level::Ok => "Claude: ",
        Level::Warn => "Claude (degraded): ",
        Level::Bad => "Claude (outage): ",
        Level::Unknown => "Claude: ",
    };
    format!("{prefix}{description}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn icon_has_transparent_corners_and_opaque_centre() {
        let px = rgba(Level::Ok);
        assert_eq!(px.len(), (SIZE * SIZE * 4) as usize);
        assert_eq!(px[3], 0, "top-left corner should be transparent");
        let mid = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;
        assert_eq!(&px[mid..mid + 4], &[0x2e, 0xa0, 0x5f, 255]);
    }

    #[test]
    fn levels_get_different_colours() {
        let mid = ((SIZE / 2) * SIZE + SIZE / 2) as usize * 4;
        let ok = rgba(Level::Ok)[mid];
        let bad = rgba(Level::Bad)[mid];
        assert_ne!(ok, bad);
    }
}
