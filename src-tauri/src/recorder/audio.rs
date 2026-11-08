//! Microphone and system (loopback) capture mixed into the encoder's PCM16 stereo stream.

use std::collections::VecDeque;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, OnceLock};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{FromSample, SampleFormat, SizedSample};
use parking_lot::Mutex;

use crate::mix::{self, OUTPUT_CHANNELS, OUTPUT_RATE};

const MIX_INTERVAL: Duration = Duration::from_millis(20);
/// A source may run ahead of the wall clock by at most this many samples before old ones are dropped.
const MAX_BACKLOG: usize = OUTPUT_RATE as usize * OUTPUT_CHANNELS / 2;
const MIC_GAIN: f32 = 1.0;
const SYSTEM_GAIN: f32 = 0.8;

/// Moment the first video frame reached the encoder; audio sample 0 is aligned to it.
pub type VideoClock = Arc<OnceLock<Instant>>;
pub type Sink = Box<dyn FnMut(&[u8]) -> Result<(), String> + Send>;

type Queue = Arc<Mutex<VecDeque<f32>>>;

struct Source {
    queue: Queue,
    gain: f32,
    _stream: cpal::Stream,
}

fn open(device_id: &str, loopback: bool, gain: f32) -> Result<Source, String> {
    let host = cpal::default_host();
    let id = device_id.parse().map_err(|_| format!("Dispositivo de áudio inválido: {device_id}"))?;
    let device = host.device_by_id(&id).ok_or("Dispositivo de áudio não encontrado.")?;
    // Building an input stream on a playback device turns on WASAPI loopback.
    let supported = if loopback { device.default_output_config() } else { device.default_input_config() }
        .map_err(|e| format!("Formato de áudio indisponível: {e}"))?;
    let queue = Queue::default();
    let config = supported.config();
    let stream = match supported.sample_format() {
        SampleFormat::F32 => build::<f32>(&device, config, queue.clone()),
        SampleFormat::I16 => build::<i16>(&device, config, queue.clone()),
        SampleFormat::I32 => build::<i32>(&device, config, queue.clone()),
        SampleFormat::U16 => build::<u16>(&device, config, queue.clone()),
        other => return Err(format!("Formato de amostra não suportado: {other}")),
    }
    .map_err(|e| format!("Falha ao abrir o áudio: {e}"))?;
    stream.play().map_err(|e| format!("Falha ao iniciar o áudio: {e}"))?;
    Ok(Source { queue, gain, _stream: stream })
}

fn build<T>(device: &cpal::Device, config: cpal::StreamConfig, queue: Queue) -> Result<cpal::Stream, cpal::Error>
where
    T: SizedSample,
    f32: FromSample<T>,
{
    let channels = usize::from(config.channels);
    let mut resampler = mix::Resampler::new(config.sample_rate);
    let (mut floats, mut stereo, mut resampled) = (Vec::new(), Vec::new(), Vec::new());
    device.build_input_stream::<T, _, _>(
        config,
        move |data: &[T], _: &cpal::InputCallbackInfo| {
            floats.clear();
            floats.extend(data.iter().map(|s| s.to_sample::<f32>()));
            mix::to_stereo(&floats, channels, &mut stereo);
            resampled.clear();
            resampler.process(&stereo, &mut resampled);
            queue.lock().extend(resampled.iter().copied());
        },
        |error| eprintln!("audio stream error: {error}"),
        None,
    )
}

/// Takes exactly `count` samples (or fewer if the source is behind) and trims excess backlog.
fn drain(queue: &Queue, count: usize, out: &mut Vec<f32>) {
    let mut queue = queue.lock();
    let excess = queue.len().saturating_sub(count + MAX_BACKLOG);
    queue.drain(..excess);
    out.clear();
    let take = count.min(queue.len());
    out.extend(queue.drain(..take));
}

pub struct AudioCapture {
    stop: Arc<AtomicBool>,
    thread: JoinHandle<Result<(), String>>,
}

impl AudioCapture {
    /// Opens the devices and starts mixing. `sink` receives PCM16 stereo at 48 kHz, paced by the wall
    /// clock from the first video frame so audio and video stay the same length.
    pub fn start(mic: Option<String>, system: Option<String>, clock: VideoClock, sink: Sink) -> Result<Self, String> {
        let stop = Arc::new(AtomicBool::new(false));
        let (ready_tx, ready_rx) = mpsc::sync_channel(1);
        let thread_stop = stop.clone();
        // cpal streams are not Send, so they live and die on this thread.
        let thread = std::thread::spawn(move || {
            let sources = mic
                .map(|id| open(&id, false, MIC_GAIN))
                .into_iter()
                .chain(system.map(|id| open(&id, true, SYSTEM_GAIN)))
                .collect::<Result<Vec<_>, _>>();
            let sources = match sources {
                Ok(sources) => {
                    let _ = ready_tx.send(Ok(()));
                    sources
                }
                Err(error) => {
                    let _ = ready_tx.send(Err(error));
                    return Ok(());
                }
            };
            run_mixer(&sources, &clock, &thread_stop, sink)
        });
        ready_rx.recv().map_err(|_| "Thread de áudio encerrou inesperadamente.".to_string())??;
        Ok(Self { stop, thread })
    }

    /// Flushes the audio due up to now and releases the devices.
    pub fn stop(self) -> Result<(), String> {
        self.stop.store(true, Ordering::Relaxed);
        self.thread.join().map_err(|_| "Thread de áudio falhou.".to_string())?
    }
}

fn run_mixer(sources: &[Source], clock: &OnceLock<Instant>, stop: &AtomicBool, mut sink: Sink) -> Result<(), String> {
    let mut sent_frames: u64 = 0;
    let mut tracks: Vec<Vec<f32>> = vec![Vec::new(); sources.len()];
    let mut pcm = Vec::new();
    loop {
        let stopping = stop.load(Ordering::Relaxed);
        match clock.get() {
            None => sources.iter().for_each(|s| s.queue.lock().clear()),
            Some(start) => {
                let due_frames = start.elapsed().as_nanos() as u64 * u64::from(OUTPUT_RATE) / 1_000_000_000;
                let samples = (due_frames.saturating_sub(sent_frames)) as usize * OUTPUT_CHANNELS;
                if samples > 0 {
                    for (source, track) in sources.iter().zip(tracks.iter_mut()) {
                        drain(&source.queue, samples, track);
                    }
                    let inputs: Vec<(&[f32], f32)> =
                        sources.iter().zip(&tracks).map(|(s, t)| (t.as_slice(), s.gain)).collect();
                    mix::mix_to_pcm16(&inputs, samples, &mut pcm);
                    sink(&pcm)?;
                    sent_frames = due_frames;
                }
            }
        }
        if stopping {
            return Ok(());
        }
        std::thread::sleep(MIX_INTERVAL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn drain_takes_what_is_available() {
        let queue = Queue::default();
        queue.lock().extend([1.0, 2.0, 3.0]);
        let mut out = Vec::new();
        drain(&queue, 2, &mut out);
        assert_eq!(out, [1.0, 2.0]);
        drain(&queue, 4, &mut out);
        assert_eq!(out, [3.0]);
    }

    #[test]
    fn drain_drops_oldest_samples_beyond_the_backlog() {
        let queue = Queue::default();
        queue.lock().extend((0..MAX_BACKLOG + 10).map(|i| i as f32));
        let mut out = Vec::new();
        drain(&queue, 4, &mut out);
        assert_eq!(out, [6.0, 7.0, 8.0, 9.0]);
        assert_eq!(queue.lock().len(), MAX_BACKLOG);
    }
}
