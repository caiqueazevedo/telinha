//! Phase 0 spike: 5 s of the primary monitor plus a synthetic 440 Hz tone into one MP4.
use std::f32::consts::TAU;
use std::time::Instant;

use windows_capture::capture::{Context, GraphicsCaptureApiHandler};
use windows_capture::encoder::{
    AudioSettingsBuilder, ContainerSettingsBuilder, VideoEncoder, VideoSettingsBuilder, VideoSettingsSubType,
};
use windows_capture::frame::Frame;
use windows_capture::graphics_capture_api::InternalCaptureControl;
use windows_capture::monitor::Monitor;
use windows_capture::settings::{
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, MinimumUpdateIntervalSettings,
    SecondaryWindowSettings, Settings,
};

const SAMPLE_RATE: u64 = 48_000;

struct Spike {
    encoder: Option<VideoEncoder>,
    start: Instant,
    samples_sent: u64,
    frames: u64,
    first_ts: Option<i64>,
    last_ts: i64,
}

impl GraphicsCaptureApiHandler for Spike {
    type Flags = (u32, u32, String);
    type Error = Box<dyn std::error::Error + Send + Sync>;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let (w, h, path) = ctx.flags;
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(w, h).sub_type(VideoSettingsSubType::H264).frame_rate(30),
            AudioSettingsBuilder::new(),
            ContainerSettingsBuilder::new(),
            path,
        )?;
        Ok(Self { encoder: Some(encoder), start: Instant::now(), samples_sent: 0, frames: 0, first_ts: None, last_ts: 0 })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, control: InternalCaptureControl) -> Result<(), Self::Error> {
        let encoder = self.encoder.as_mut().unwrap();
        encoder.send_frame(frame)?;
        let ts = frame.timestamp()?.Duration;
        self.first_ts.get_or_insert(ts);
        self.last_ts = ts;
        self.frames += 1;

        let due = (self.start.elapsed().as_secs_f64() * SAMPLE_RATE as f64) as u64;
        if due > self.samples_sent {
            let mut pcm = Vec::with_capacity(((due - self.samples_sent) * 4) as usize);
            for n in self.samples_sent..due {
                let v = ((n as f32 * 440.0 / SAMPLE_RATE as f32 * TAU).sin() * 0.3 * i16::MAX as f32) as i16;
                pcm.extend_from_slice(&v.to_le_bytes());
                pcm.extend_from_slice(&v.to_le_bytes());
            }
            encoder.send_audio_buffer(&pcm, 0)?;
            self.samples_sent = due;
        }

        if self.start.elapsed().as_secs() >= 5 {
            println!("frames={} samples={} ts_span={}ms wall={:?}", self.frames, self.samples_sent, (self.last_ts - self.first_ts.unwrap()) / 10_000, self.start.elapsed());
            self.encoder.take().unwrap().finish()?;
            println!("finished at {:?}", self.start.elapsed());
            control.stop();
        }
        Ok(())
    }
}

fn main() {
    let path = std::env::args().nth(1).expect("usage: spike_av <out.mp4>");
    let monitor = Monitor::primary().expect("primary monitor");
    let w = monitor.width().unwrap() & !1;
    let h = monitor.height().unwrap() & !1;
    println!("monitor {w}x{h} -> {path}");
    let settings = Settings::new(
        monitor,
        CursorCaptureSettings::WithCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        (w, h, path),
    );
    Spike::start(settings).expect("capture failed");
    println!("done");
}
