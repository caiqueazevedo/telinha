//! Pure audio helpers: channel conversion, resampling to the encoder rate and mixing to PCM16.

pub const OUTPUT_RATE: u32 = 48_000;
pub const OUTPUT_CHANNELS: usize = 2;

/// Converts interleaved frames with `channels` channels to interleaved stereo.
pub fn to_stereo(input: &[f32], channels: usize, out: &mut Vec<f32>) {
    out.clear();
    match channels {
        0 => {}
        1 => input.iter().for_each(|&s| out.extend_from_slice(&[s, s])),
        _ => input.chunks_exact(channels).for_each(|frame| out.extend_from_slice(&frame[..2])),
    }
}

/// Streaming linear resampler for interleaved stereo. Keeps state across calls so
/// consecutive device buffers join without clicks.
pub struct Resampler {
    step: f64,
    pos: f64,
    prev: [f32; 2],
}

impl Resampler {
    pub fn new(from_rate: u32) -> Self {
        Self { step: f64::from(from_rate) / f64::from(OUTPUT_RATE), pos: 0.0, prev: [0.0; 2] }
    }

    /// Appends the resampled frames of `input` (stereo) to `out`.
    pub fn process(&mut self, input: &[f32], out: &mut Vec<f32>) {
        let frames = input.len() / OUTPUT_CHANNELS;
        if frames == 0 {
            return;
        }
        // Frame -1 is the last frame of the previous call.
        let frame = |i: isize| -> [f32; 2] {
            if i < 0 { self.prev } else { [input[i as usize * 2], input[i as usize * 2 + 1]] }
        };
        let last = frames as isize - 1;
        while (self.pos.floor() as isize) < last {
            let index = self.pos.floor() as isize;
            let frac = (self.pos - index as f64) as f32;
            let (a, b) = (frame(index), frame(index + 1));
            out.push(a[0] + (b[0] - a[0]) * frac);
            out.push(a[1] + (b[1] - a[1]) * frac);
            self.pos += self.step;
        }
        self.prev = frame(last);
        self.pos -= frames as f64;
    }
}

/// Sums tracks with their gains and writes little-endian PCM16. Tracks shorter than
/// `samples` are padded with silence.
pub fn mix_to_pcm16(tracks: &[(&[f32], f32)], samples: usize, out: &mut Vec<u8>) {
    out.clear();
    out.reserve(samples * 2);
    for i in 0..samples {
        let sum: f32 = tracks.iter().map(|(track, gain)| track.get(i).copied().unwrap_or(0.0) * gain).sum();
        let value = (sum.clamp(-1.0, 1.0) * f32::from(i16::MAX)) as i16;
        out.extend_from_slice(&value.to_le_bytes());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pcm(bytes: &[u8]) -> Vec<i16> {
        bytes.chunks_exact(2).map(|b| i16::from_le_bytes([b[0], b[1]])).collect()
    }

    #[test]
    fn mono_is_duplicated_to_both_channels() {
        let mut out = Vec::new();
        to_stereo(&[0.1, 0.2], 1, &mut out);
        assert_eq!(out, [0.1, 0.1, 0.2, 0.2]);
    }

    #[test]
    fn surround_keeps_front_left_and_right() {
        let mut out = Vec::new();
        to_stereo(&[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], 3, &mut out);
        assert_eq!(out, [1.0, 2.0, 4.0, 5.0]);
    }

    #[test]
    fn same_rate_passes_frames_through_with_one_frame_delay() {
        let mut resampler = Resampler::new(OUTPUT_RATE);
        let mut out = Vec::new();
        resampler.process(&[1.0, 1.0, 2.0, 2.0, 3.0, 3.0], &mut out);
        resampler.process(&[4.0, 4.0], &mut out);
        assert_eq!(out, [1.0, 1.0, 2.0, 2.0, 3.0, 3.0]);
    }

    #[test]
    fn resampling_44100_produces_48000_frames_per_second() {
        let mut resampler = Resampler::new(44_100);
        let mut out = Vec::new();
        let chunk = vec![0.5; 441 * 2];
        for _ in 0..100 {
            resampler.process(&chunk, &mut out);
        }
        let frames = out.len() / 2;
        assert!((47_990..=48_000).contains(&frames), "got {frames} frames");
        assert!(out.iter().all(|&s| (s - 0.5).abs() < 1e-6));
    }

    #[test]
    fn resampling_interpolates_between_frames() {
        let mut resampler = Resampler::new(OUTPUT_RATE / 2);
        let mut out = Vec::new();
        resampler.process(&[0.0, 0.0, 1.0, 1.0, 2.0, 2.0], &mut out);
        assert_eq!(out, [0.0, 0.0, 0.5, 0.5, 1.0, 1.0, 1.5, 1.5]);
    }

    #[test]
    fn mix_sums_with_gain_and_pads_short_tracks() {
        let mut out = Vec::new();
        mix_to_pcm16(&[(&[0.5, 0.5], 1.0), (&[0.25], 0.5)], 3, &mut out);
        let max = f32::from(i16::MAX);
        assert_eq!(pcm(&out), [(0.625 * max) as i16, (0.5 * max) as i16, 0]);
    }

    #[test]
    fn mix_clips_instead_of_wrapping() {
        let mut out = Vec::new();
        mix_to_pcm16(&[(&[0.9, -0.9], 1.0), (&[0.9, -0.9], 1.0)], 2, &mut out);
        assert_eq!(pcm(&out), [i16::MAX, -i16::MAX]);
    }
}
