use std::path::{Path, PathBuf};
use std::sync::{Arc, OnceLock};
use std::time::Instant;

use windows_capture::capture::{CaptureControl, Context, GraphicsCaptureApiHandler};
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

use super::audio::{AudioCapture, VideoClock};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const FRAME_RATE: u32 = 30;

pub struct EncoderConfig {
    width: u32,
    height: u32,
    path: PathBuf,
    audio: bool,
    clock: VideoClock,
}

/// Devices to record; `None` leaves that source out.
#[derive(Debug, Default, Clone, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioOptions {
    pub mic: Option<String>,
    pub system: Option<String>,
}

impl AudioOptions {
    fn any(&self) -> bool {
        self.mic.is_some() || self.system.is_some()
    }
}

/// Receives captured frames on the capture thread and feeds them to the encoder.
pub struct Capture {
    encoder: Option<VideoEncoder>,
    clock: VideoClock,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = EncoderConfig;
    type Error = BoxError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let EncoderConfig { width, height, path, audio, clock } = ctx.flags;
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height).sub_type(VideoSettingsSubType::H264).frame_rate(FRAME_RATE),
            AudioSettingsBuilder::new().disabled(!audio),
            ContainerSettingsBuilder::new(),
            path,
        )?;
        Ok(Self { encoder: Some(encoder), clock })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _control: InternalCaptureControl) -> Result<(), Self::Error> {
        if let Some(encoder) = self.encoder.as_mut() {
            encoder.send_frame(frame)?;
            self.clock.get_or_init(Instant::now);
        }
        Ok(())
    }
}

/// A running full-screen recording of the primary monitor.
pub struct Session {
    control: CaptureControl<Capture, BoxError>,
    handler: Arc<parking_lot::Mutex<Capture>>,
    audio: Option<AudioCapture>,
}

impl Session {
    pub fn start(path: &Path, audio: AudioOptions) -> Result<Self, String> {
        let monitor = Monitor::primary().map_err(|e| format!("Monitor principal indisponível: {e}"))?;
        let width = monitor.width().map_err(|e| e.to_string())? & !1;
        let height = monitor.height().map_err(|e| e.to_string())? & !1;
        let clock = VideoClock::new(OnceLock::new());
        let settings = Settings::new(
            monitor,
            CursorCaptureSettings::WithCursor,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            EncoderConfig { width, height, path: path.to_path_buf(), audio: audio.any(), clock: clock.clone() },
        );
        let control = Capture::start_free_threaded(settings).map_err(|e| format!("Falha ao iniciar a captura: {e}"))?;
        let handler = control.callback();
        let capture = if audio.any() {
            let sink_handler = handler.clone();
            let sink = Box::new(move |pcm: &[u8]| match sink_handler.lock().encoder.as_mut() {
                Some(encoder) => encoder.send_audio_buffer(pcm, 0).map_err(|e| e.to_string()),
                None => Ok(()),
            });
            match AudioCapture::start(audio.mic, audio.system, clock, sink) {
                Ok(capture) => Some(capture),
                Err(error) => {
                    let _ = control.stop();
                    return Err(error);
                }
            }
        } else {
            None
        };
        Ok(Self { control, handler, audio: capture })
    }

    /// Stops capturing and finalizes the MP4. Blocks until the file is written.
    pub fn stop(self) -> Result<(), String> {
        let audio = self.audio.map_or(Ok(()), AudioCapture::stop);
        self.control.stop().map_err(|e| format!("Falha ao parar a captura: {e}"))?;
        let encoder = self.handler.lock().encoder.take().ok_or("Encoder já finalizado.")?;
        encoder.finish().map_err(|e| format!("Falha ao finalizar o vídeo: {e}"))?;
        audio
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real display: `cargo test -- --ignored records_primary_monitor`.
    /// Set `TELINHA_PROBE_AUDIO=1` to also record the default mic and speakers.
    #[test]
    #[ignore]
    fn records_primary_monitor() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("probe.mp4");
        let _ = std::fs::remove_file(&path);
        let audio = std::env::var("TELINHA_PROBE_AUDIO").map_or_else(
            |_| AudioOptions::default(),
            |_| {
                use cpal::traits::{DeviceTrait, HostTrait};
                let host = cpal::default_host();
                let id = |d: Option<cpal::Device>| d.and_then(|d| d.id().ok()).map(|id| id.to_string());
                AudioOptions { mic: id(host.default_input_device()), system: id(host.default_output_device()) }
            },
        );
        let session = Session::start(&path, audio).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(4));
        session.stop().unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }
}
