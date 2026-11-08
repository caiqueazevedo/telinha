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
    ColorFormat, CursorCaptureSettings, DirtyRegionSettings, DrawBorderSettings, GraphicsCaptureItemType,
    MinimumUpdateIntervalSettings, SecondaryWindowSettings, Settings,
};
use windows_capture::window::Window;

use super::audio::{AudioCapture, VideoClock};
use crate::region::{self, Crop, Selection};

type BoxError = Box<dyn std::error::Error + Send + Sync>;

const FRAME_RATE: u32 = 30;
const MIN_BITRATE: u64 = 8_000_000;
const MAX_BITRATE: u64 = 50_000_000;

/// Bitrate for the encoded size: the encoder's fixed default starves large frames, and Media
/// Foundation undershoots badly on small ones. About 0.25 bit per pixel keeps screen text sharp.
fn bitrate_for(width: u32, height: u32, fps: u32) -> u32 {
    let bits = u64::from(width) * u64::from(height) * u64::from(fps) / 4;
    bits.clamp(MIN_BITRATE, MAX_BITRATE) as u32
}

pub struct EncoderConfig {
    width: u32,
    height: u32,
    path: PathBuf,
    audio: bool,
    clock: VideoClock,
    crop: Option<Crop>,
    scale: u32,
}

/// What to record. Monitors use the one-based ids from `list_sources`.
#[derive(Debug, Clone, serde::Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Target {
    Monitor { monitor: usize },
    Window { id: String },
    Region {
        monitor: usize,
        #[serde(flatten)]
        selection: Selection,
    },
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
    crop: Option<Crop>,
    scale: usize,
    unpadded: Vec<u8>,
    flipped: Vec<u8>,
}

impl GraphicsCaptureApiHandler for Capture {
    type Flags = EncoderConfig;
    type Error = BoxError;

    fn new(ctx: Context<Self::Flags>) -> Result<Self, Self::Error> {
        let EncoderConfig { width, height, path, audio, clock, crop, scale } = ctx.flags;
        let video = VideoSettingsBuilder::new(width, height)
            .sub_type(VideoSettingsSubType::H264)
            .frame_rate(FRAME_RATE)
            .bitrate(bitrate_for(width, height, FRAME_RATE));
        let encoder = VideoEncoder::new(
            video,
            AudioSettingsBuilder::new().disabled(!audio),
            ContainerSettingsBuilder::new(),
            path,
        )?;
        Ok(Self { encoder: Some(encoder), clock, crop, scale: scale as usize, unpadded: Vec::new(), flipped: Vec::new() })
    }

    fn on_frame_arrived(&mut self, frame: &mut Frame, _control: InternalCaptureControl) -> Result<(), Self::Error> {
        let Self { encoder, clock, crop, scale, unpadded, flipped } = self;
        let Some(encoder) = encoder.as_mut() else { return Ok(()) };
        match crop {
            None => encoder.send_frame(frame)?,
            // A frame smaller than the crop (display mode change) is dropped rather than encoded wrong.
            Some(crop) if !crop.fits(frame.width(), frame.height()) => return Ok(()),
            Some(crop) => {
                let timestamp = frame.timestamp()?.Duration;
                let buffer = frame.buffer_crop(crop.x, crop.y, crop.right(), crop.bottom())?;
                let pixels = buffer.as_nopadding_buffer(unpadded);
                region::upscale_flip(pixels, crop.width as usize, *scale, flipped);
                encoder.send_frame_buffer(flipped, timestamp)?;
            }
        }
        clock.get_or_init(Instant::now);
        Ok(())
    }
}

fn monitor(id: usize) -> Result<Monitor, String> {
    Monitor::from_index(id).map_err(|_| "Monitor não encontrado. Atualize a lista de fontes.".to_string())
}

fn monitor_size(monitor: &Monitor) -> Result<(u32, u32), String> {
    Ok((monitor.width().map_err(|e| e.to_string())?, monitor.height().map_err(|e| e.to_string())?))
}

fn launch<T>(item: T, config: EncoderConfig) -> Result<CaptureControl<Capture, BoxError>, String>
where
    T: TryInto<GraphicsCaptureItemType> + Send + 'static,
{
    let settings = Settings::new(
        item,
        CursorCaptureSettings::WithCursor,
        DrawBorderSettings::WithoutBorder,
        SecondaryWindowSettings::Default,
        MinimumUpdateIntervalSettings::Default,
        DirtyRegionSettings::Default,
        ColorFormat::Bgra8,
        config,
    );
    Capture::start_free_threaded(settings).map_err(|e| format!("Falha ao iniciar a captura: {e}"))
}

fn window_side(value: Result<i32, windows_capture::window::Error>) -> Result<u32, String> {
    value.ok().and_then(|v| u32::try_from(v).ok()).ok_or_else(|| "Tamanho da janela indisponível.".into())
}

/// A running recording of a monitor, a window or a region of a monitor.
pub struct Session {
    control: CaptureControl<Capture, BoxError>,
    handler: Arc<parking_lot::Mutex<Capture>>,
    audio: Option<AudioCapture>,
}

impl Session {
    pub fn start(path: &Path, target: Target, audio: AudioOptions) -> Result<Self, String> {
        let clock = VideoClock::new(OnceLock::new());
        let config = |width: u32, height: u32, crop: Option<Crop>, scale: u32| EncoderConfig {
            width: (width & !1) * scale,
            height: (height & !1) * scale,
            path: path.to_path_buf(),
            audio: audio.any(),
            clock: clock.clone(),
            crop,
            scale,
        };
        let control = match target {
            Target::Monitor { monitor: id } => {
                let monitor = monitor(id)?;
                let (width, height) = monitor_size(&monitor)?;
                launch(monitor, config(width, height, None, 1))?
            }
            Target::Window { id } => {
                let hwnd: usize = id.parse().map_err(|_| "Janela inválida.".to_string())?;
                let window = Window::from_raw_hwnd(hwnd as *mut std::ffi::c_void);
                if !window.is_valid() {
                    return Err("A janela não está mais disponível. Atualize a lista de fontes.".into());
                }
                // The encoder pads or crops frames when the window is resized mid-recording.
                let (width, height) = (window_side(window.width())?, window_side(window.height())?);
                launch(window, config(width, height, None, 1))?
            }
            Target::Region { monitor: id, selection } => {
                let monitor = monitor(id)?;
                let (width, height) = monitor_size(&monitor)?;
                let crop = region::fit(selection, width, height).ok_or("A região selecionada é pequena demais.")?;
                let scale = region::output_scale(crop.width, crop.height);
                launch(monitor, config(crop.width, crop.height, Some(crop), scale))?
            }
        };
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
        record(Target::Monitor { monitor: 1 }, "probe.mp4");
    }

    /// `cargo test -- --ignored records_region` writes `probe-region.mp4`: 640x360 at 100,100, encoded at 1920x1080.
    #[test]
    #[ignore]
    fn records_region() {
        let selection = Selection { x: 100, y: 100, width: 640, height: 360 };
        record(Target::Region { monitor: 1, selection }, "probe-region.mp4");
    }

    #[test]
    fn bitrate_grows_with_the_encoded_size() {
        assert_eq!(bitrate_for(1920, 1080, 30), 15_552_000);
        assert!(bitrate_for(2560, 1440, 30) > bitrate_for(1920, 1080, 30));
    }

    #[test]
    fn bitrate_is_kept_between_floor_and_ceiling() {
        assert_eq!(bitrate_for(640, 360, 30), 8_000_000);
        assert_eq!(bitrate_for(3840, 2160, 60), 50_000_000);
    }

    #[test]
    fn target_deserializes_from_the_front_end_shape() {
        let json = r#"{"kind":"region","monitor":2,"x":10,"y":20,"width":300,"height":200}"#;
        let target: Target = serde_json::from_str(json).unwrap();
        assert!(matches!(
            target,
            Target::Region { monitor: 2, selection: Selection { x: 10, y: 20, width: 300, height: 200 } }
        ));
        let target: Target = serde_json::from_str(r#"{"kind":"window","id":"1234"}"#).unwrap();
        assert!(matches!(target, Target::Window { id } if id == "1234"));
    }

    fn record(target: Target, name: &str) {
        let path = Path::new(env!("CARGO_MANIFEST_DIR")).join("target").join(name);
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
        let session = Session::start(&path, target, audio).unwrap();
        std::thread::sleep(std::time::Duration::from_secs(4));
        session.stop().unwrap();
        assert!(std::fs::metadata(&path).unwrap().len() > 0);
    }
}
