use cpal::traits::{DeviceTrait, HostTrait};
use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AudioDevice {
    pub id: String,
    pub name: String,
    pub is_default: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AudioDevices {
    pub inputs: Vec<AudioDevice>,
    pub outputs: Vec<AudioDevice>,
}

fn describe(devices: impl Iterator<Item = cpal::Device>, default: Option<cpal::Device>) -> Vec<AudioDevice> {
    let default_id = default.and_then(|d| d.id().ok()).map(|id| id.to_string());
    devices
        .filter_map(|device| {
            let id = device.id().ok()?.to_string();
            let name = device.description().ok()?.name().to_string();
            Some(AudioDevice { is_default: default_id.as_deref() == Some(id.as_str()), id, name })
        })
        .collect()
}

/// Lists microphones and playback devices (the latter are recorded through WASAPI loopback).
#[tauri::command]
pub fn list_audio_devices() -> Result<AudioDevices, String> {
    let host = cpal::default_host();
    let inputs = host.input_devices().map_err(|e| e.to_string())?;
    let outputs = host.output_devices().map_err(|e| e.to_string())?;
    Ok(AudioDevices {
        inputs: describe(inputs, host.default_input_device()),
        outputs: describe(outputs, host.default_output_device()),
    })
}
