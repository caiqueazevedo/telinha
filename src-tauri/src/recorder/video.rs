use std::path::{Path, PathBuf};
use std::sync::Arc;

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

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const FRAME_RATE: u32 = 30;

pub struct EncoderConfig {
    width: u32,
    height: u32,
    path: PathBuf,
}

/// Receives captured frames on the capture thread and feeds them to the encoder.
pub struct Capture {
    encoder: Option<VideoEncoder>,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = EncoderConfig;
    type Error = BoxError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let EncoderConfig { width, height, path } = ctx.flags;
        let encoder = VideoEncoder::new(
            VideoSettingsBuilder::new(width, height).sub_type(VideoSettingsSubType::H264).frame_rate(FRAME_RATE),
            AudioSettingsBuilder::new().disabled(true),
            ContainerSettingsBuilder::new(),
            path,
        )?;
        Ok(Self { encoder: Some(encoder) })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _control: InternalCaptureControl) -> Result<(), Self::Error> {
        if let Some(encoder) = self.encoder.as_mut() {
            encoder.send_frame(frame)?;
        }
        Ok(())
    }
}

/// A running full-screen recording of the primary monitor.
pub struct Session {
    control: CaptureControl<Capture, BoxError>,
    handler: Arc<parking_lot::Mutex<Capture>>,
}

impl Session {
    pub fn start(path: &Path) -> Result<Self, String> {
        let monitor = Monitor::primary().map_err(|e| format!("Monitor principal indisponível: {e}"))?;
        let width = monitor.width().map_err(|e| e.to_string())? & !1;
        let height = monitor.height().map_err(|e| e.to_string())? & !1;
        let settings = Settings::new(
            monitor,
            CursorCaptureSettings::WithCursor,
            DrawBorderSettings::WithoutBorder,
            SecondaryWindowSettings::Default,
            MinimumUpdateIntervalSettings::Default,
            DirtyRegionSettings::Default,
            ColorFormat::Bgra8,
            EncoderConfig { width, height, path: path.to_path_buf() },
        );
        let control = Capture::start_free_threaded(settings).map_err(|e| format!("Falha ao iniciar a captura: {e}"))?;
        let handler = control.callback();
        Ok(Self { control, handler })
    }

    /// Stops capturing and finalizes the MP4. Blocks until the file is written.
    pub fn stop(self) -> Result<(), String> {
        self.control.stop().map_err(|e| format!("Falha ao parar a captura: {e}"))?;
        let encoder = self.handler.lock().encoder.take().ok_or("Encoder já finalizado.")?;
        encoder.finish().map_err(|e| format!("Falha ao finalizar o vídeo: {e}"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real display: `cargo test -- --ignored records_primary_monitor`.
    #[test]
    #[ignore]
    fn records_primary_monitor() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join("probe.mp4");
        let _ = std::fs::remove_file(&path);
        let session = Session::start(&path).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(4));
        session.stop().unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }
}
