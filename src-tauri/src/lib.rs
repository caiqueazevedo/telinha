mod devices;
mod mix;
mod paths;
mod picker;
mod recorder;
mod region;
mod sources;
mod webcam;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(recorder::Recorder::default())
        .manage(picker::Picker::default())
        .invoke_handler(tauri::generate_handler![
            recorder::recording_status,
            recorder::start_recording,
            recorder::stop_recording,
            recorder::open_output_dir,
            devices::list_audio_devices,
            sources::list_sources,
            picker::open_region_picker,
            picker::finish_region_pick,
            webcam::open_webcam,
            webcam::close_webcam,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
