//! Pure geometry for region capture: fitting a selection to the monitor and preparing pixels
//! for the encoder.

/// Smallest side the encoder accepts for a region.
pub const MIN_SIDE: u32 = 16;

/// A selection in physical pixels, relative to the monitor's top-left corner.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub struct Selection {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

/// A crop guaranteed to fit inside the monitor, with even sides (H.264 needs them).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Crop {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
}

impl Crop {
    pub const fn right(&self) -> u32 {
        self.x + self.width
    }

    pub const fn bottom(&self) -> u32 {
        self.y + self.height
    }

    pub const fn fits(&self, width: u32, height: u32) -> bool {
        self.right() <= width && self.bottom() <= height
    }
}

/// Fits `selection` inside a `bounds_width` x `bounds_height` monitor. Returns `None` when what
/// is left is smaller than [`MIN_SIDE`].
pub fn fit(selection: Selection, bounds_width: u32, bounds_height: u32) -> Option<Crop> {
    let axis = |start: i32, length: u32, bound: u32| -> (u32, u32) {
        let begin = i64::from(start).clamp(0, i64::from(bound));
        let end = (i64::from(start) + i64::from(length)).clamp(0, i64::from(bound));
        (begin as u32, ((end - begin) as u32) & !1)
    };
    let (x, width) = axis(selection.x, selection.width, bounds_width);
    let (y, height) = axis(selection.y, selection.height, bounds_height);
    (width >= MIN_SIDE && height >= MIN_SIDE).then_some(Crop { x, y, width, height })
}

/// Integer factor a region is enlarged by before encoding, so a small selection still reaches about
/// 1080 lines (players would otherwise blur it when filling the screen) without passing 4K.
pub fn output_scale(width: u32, height: u32) -> u32 {
    1080u32.div_ceil(height).min(3840 / width).min(2160 / height).max(1)
}

/// Enlarges a tightly packed BGRA image by `scale` with nearest-neighbour (text stays crisp) and
/// reverses its rows: the encoder's raw-buffer path expects bottom-to-top rows, while captured
/// frames are top-to-bottom.
pub fn upscale_flip(pixels: &[u8], width: usize, scale: usize, out: &mut Vec<u8>) {
    out.clear();
    out.reserve(pixels.len() * scale * scale);
    let row_bytes = width * 4;
    for row in pixels.chunks_exact(row_bytes).rev() {
        let start = out.len();
        for pixel in row.chunks_exact(4) {
            (0..scale).for_each(|_| out.extend_from_slice(pixel));
        }
        (1..scale).for_each(|_| out.extend_from_within(start..start + row_bytes * scale));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn selection(x: i32, y: i32, width: u32, height: u32) -> Selection {
        Selection { x, y, width, height }
    }

    #[test]
    fn selection_inside_the_monitor_is_kept() {
        assert_eq!(fit(selection(100, 50, 640, 360), 1920, 1080), Some(Crop { x: 100, y: 50, width: 640, height: 360 }));
    }

    #[test]
    fn odd_sides_are_rounded_down_to_even() {
        assert_eq!(fit(selection(0, 0, 641, 361), 1920, 1080), Some(Crop { x: 0, y: 0, width: 640, height: 360 }));
    }

    #[test]
    fn selection_past_the_edges_is_clamped() {
        assert_eq!(fit(selection(-20, 1000, 200, 200), 1920, 1080), Some(Crop { x: 0, y: 1000, width: 180, height: 80 }));
    }

    #[test]
    fn selection_outside_the_monitor_is_rejected() {
        assert_eq!(fit(selection(2000, 0, 300, 300), 1920, 1080), None);
    }

    #[test]
    fn tiny_selection_is_rejected() {
        assert_eq!(fit(selection(10, 10, 15, 400), 1920, 1080), None);
    }

    #[test]
    fn crop_reports_whether_it_fits_a_frame() {
        let crop = Crop { x: 100, y: 100, width: 200, height: 100 };
        assert!(crop.fits(300, 200));
        assert!(!crop.fits(299, 200));
    }

    #[test]
    fn small_regions_are_scaled_up_to_about_1080_lines() {
        assert_eq!(output_scale(532, 442), 3);
        assert_eq!(output_scale(640, 360), 3);
    }

    #[test]
    fn large_regions_keep_their_size() {
        assert_eq!(output_scale(1920, 1080), 1);
        assert_eq!(output_scale(2560, 1440), 1);
    }

    #[test]
    fn scaling_never_passes_4k() {
        assert_eq!(output_scale(3000, 200), 1);
        assert_eq!(output_scale(1000, 300), 3);
    }

    #[test]
    fn rows_are_reversed_at_scale_one() {
        let mut out = Vec::new();
        upscale_flip(&[1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3], 1, 1, &mut out);
        assert_eq!(out, [3, 3, 3, 3, 2, 2, 2, 2, 1, 1, 1, 1]);
    }

    #[test]
    fn pixels_are_repeated_across_and_down_after_flipping() {
        // Two rows of two pixels: [a b] over [c d].
        let (a, b, c, d) = ([1u8; 4], [2u8; 4], [3u8; 4], [4u8; 4]);
        let image = [a, b, c, d].concat();
        let mut out = Vec::new();
        upscale_flip(&image, 2, 2, &mut out);
        let bottom = [c, c, d, d].concat();
        let top = [a, a, b, b].concat();
        assert_eq!(out, [&bottom[..], &bottom, &top, &top].concat());
    }
}
