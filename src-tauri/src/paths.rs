use std::path::{Path, PathBuf};

use chrono::NaiveDateTime;

pub const OUTPUT_SUBDIR: &str = "Telinha";

pub fn file_stem(at: NaiveDateTime) -> String {
    format!("telinha-{}", at.format("%Y%m%d-%H%M%S"))
}

/// Picks a path in `dir` that does not exist yet, suffixing `-2`, `-3`, ... on collision.
pub fn unique_path(dir: &Path, stem: &str, exists: impl Fn(&Path) -> bool) -> PathBuf {
    let first = dir.join(format!("{stem}.mp4"));
    if !exists(&first) {
        return first;
    }
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}.mp4")))
        .find(|candidate| !exists(candidate))
        .expect("unbounded range always yields a free name")
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::NaiveDate;

    fn at(h: u32, m: u32, s: u32) -> NaiveDateTime {
        NaiveDate::from_ymd_opt(2026, 3, 7).unwrap().and_hms_opt(h, m, s).unwrap()
    }

    #[test]
    fn file_stem_is_zero_padded_timestamp() {
        assert_eq!(file_stem(at(9, 5, 3)), "telinha-20260307-090503");
    }

    #[test]
    fn unique_path_returns_plain_name_when_free() {
        let dir = Path::new("C:/videos");
        assert_eq!(unique_path(dir, "rec", |_| false), dir.join("rec.mp4"));
    }

    #[test]
    fn unique_path_suffixes_until_free() {
        let dir = Path::new("C:/videos");
        let taken = [dir.join("rec.mp4"), dir.join("rec-2.mp4")];
        let path = unique_path(dir, "rec", |p| taken.iter().any(|t| t == p));
        assert_eq!(path, dir.join("rec-3.mp4"));
    }
}
