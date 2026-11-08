# Changelog

## [1.0.0] - 2026-11-07

First release of the Windows recorder MVP.

### Added
- Full screen recording to MP4 (H.264 + AAC) with Windows Graphics Capture and Media Foundation.
- Window and region capture modes, with a region picker overlay.
- Mixed microphone and system audio, each with its own device choice.
- Floating webcam bubble over the recording.
- Global shortcut `Ctrl+Shift+R` to start and stop recording.
- Recorder windows excluded from the capture.
- Recent recordings list and a button to open the output folder.
- telinha logo and app icons.

### Fixed
- Audio pops removed with a jitter buffer.
- Small regions upscaled before encoding, with bitrate scaled to the output size.
