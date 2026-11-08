//! Jitter buffer between a capture callback and the mixer clock: absorbs uneven device packets so the
//! mixer never reads a half-filled queue, and nudges the fill level to follow clock drift.

use std::collections::VecDeque;

pub struct JitterBuffer {
    queue: VecDeque<f32>,
    channels: usize,
    target: usize,
    tolerance: usize,
    max: usize,
    primed: bool,
}

impl JitterBuffer {
    /// Sizes are in interleaved samples: the buffer refills to `target` after an underrun, corrects
    /// by one frame per pull once the level leaves `target ± tolerance`, and cuts back to `target`
    /// above `max`.
    pub fn new(channels: usize, target: usize, tolerance: usize, max: usize) -> Self {
        Self { queue: VecDeque::with_capacity(max), channels, target, tolerance, max, primed: false }
    }

    pub fn push(&mut self, samples: &[f32]) {
        self.queue.extend(samples);
    }

    pub fn clear(&mut self) {
        self.queue.clear();
        self.primed = false;
    }

    /// Writes exactly `count` samples to `out`, silence while the buffer is refilling.
    pub fn pull(&mut self, count: usize, out: &mut Vec<f32>) {
        out.clear();
        let frame = self.channels;
        if self.queue.len() > self.max {
            let excess = (self.queue.len() - self.target) / frame * frame;
            self.queue.drain(..excess);
        }
        let level = self.queue.len();
        if !self.primed {
            if level < self.target {
                out.resize(count, 0.0);
                return;
            }
            self.primed = true;
        }
        if level < count {
            out.extend(self.queue.drain(..));
            out.resize(count, 0.0);
            self.primed = false;
            return;
        }
        if level > self.target + self.tolerance && level - frame >= count {
            self.queue.drain(..frame);
        } else if level < self.target.saturating_sub(self.tolerance) && count > frame {
            out.extend(self.queue.drain(..count - frame));
            out.extend_from_within(out.len() - frame..);
            return;
        }
        out.extend(self.queue.drain(..count));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Stereo, refill at 8 samples, ±2 tolerance, cut above 20.
    fn buffer() -> JitterBuffer {
        JitterBuffer::new(2, 8, 2, 20)
    }

    fn ramp(from: usize, to: usize) -> Vec<f32> {
        (from..to).map(|i| i as f32).collect()
    }

    #[test]
    fn plays_silence_until_the_target_is_buffered() {
        let mut jitter = buffer();
        let mut out = Vec::new();
        jitter.push(&ramp(0, 6));
        jitter.pull(4, &mut out);
        assert_eq!(out, [0.0; 4]);

        jitter.push(&ramp(6, 8));
        jitter.pull(4, &mut out);
        assert_eq!(out, ramp(0, 4));
    }

    #[test]
    fn delivers_continuously_once_primed() {
        let mut jitter = buffer();
        let mut out = Vec::new();
        jitter.push(&ramp(0, 8));
        jitter.pull(4, &mut out);
        jitter.push(&ramp(8, 12));
        jitter.pull(4, &mut out);
        assert_eq!(out, ramp(4, 8));
    }

    #[test]
    fn underrun_pads_with_silence_and_refills_before_playing_again() {
        let mut jitter = buffer();
        let mut out = Vec::new();
        jitter.push(&ramp(0, 8));
        jitter.pull(6, &mut out);
        jitter.pull(4, &mut out);
        assert_eq!(out, [6.0, 7.0, 0.0, 0.0]);

        jitter.push(&ramp(8, 12));
        jitter.pull(4, &mut out);
        assert_eq!(out, [0.0; 4], "must wait for the target again");
    }

    #[test]
    fn drops_one_frame_when_running_ahead() {
        let mut jitter = buffer();
        let mut out = Vec::new();
        jitter.push(&ramp(0, 12));
        jitter.pull(4, &mut out);
        assert_eq!(out, ramp(2, 6));
    }

    #[test]
    fn repeats_one_frame_when_running_behind() {
        let mut jitter = buffer();
        let mut out = Vec::new();
        jitter.push(&ramp(0, 8));
        jitter.pull(4, &mut out);
        jitter.pull(4, &mut out);
        assert_eq!(out, [4.0, 5.0, 4.0, 5.0]);
    }

    #[test]
    fn cuts_a_large_backlog_back_to_the_target() {
        let mut jitter = buffer();
        let mut out = Vec::new();
        jitter.push(&ramp(0, 30));
        jitter.pull(4, &mut out);
        assert_eq!(out, ramp(22, 26));
    }
}
