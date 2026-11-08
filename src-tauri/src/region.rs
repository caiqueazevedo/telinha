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

/// Reverses the row order of a tightly packed image. The encoder's raw-buffer path expects
/// bottom-to-top rows, while captured frames are top-to-bottom.
pub fn flip_rows(pixels: &[u8], row_bytes: usize, out: &mut Vec<u8>) {
    out.clear();
    if row_bytes == 0 {
        return;
    }
    out.reserve(pixels.len());
    pixels.chunks_exact(row_bytes).rev().for_each(|row| out.extend_from_slice(row));
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
    fn rows_are_reversed() {
        let mut out = Vec::new();
        flip_rows(&[1, 2, 3, 4, 5, 6], 2, &mut out);
        assert_eq!(out, [5, 6, 3, 4, 1, 2]);
    }
}
