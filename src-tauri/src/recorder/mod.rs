pub mod state;
mod video;

use std::path::{Path, PathBuf};
use std::sync::Mutex;

use tauri::{AppHandle, Emitter, Manager, State};
use tauri_plugin_opener::OpenerExt;

use crate::paths;
use state::{Machine, Status};
use video::Session;

pub const STATE_EVENT: &str = "recording://state";

#[derive(Default)]
pub struct Recorder {
    machine: Mutex<Machine>,
    session: Mutex<Option<Session>>,
}

fn output_dir(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app.path().video_dir().map_err(|e| e.to_string())?.join(paths::OUTPUT_SUBDIR);
    std::fs::create_dir_all(&dir).map_err(|e| format!("Não foi possível criar {}: {e}", dir.display()))?;
    Ok(dir)
}

fn publish(app: &AppHandle, status: Status) -> Status {
    let _ = app.emit(STATE_EVENT, &status);
    status
}

#[tauri::command]
pub fn recording_status(recorder: State<'_, Recorder>) -> Status {
    recorder.machine.lock().unwrap().status()
}

#[tauri::command]
pub async fn start_recording(app: AppHandle, recorder: State<'_, Recorder>) -> Result<Status, String> {
    let mut machine = recorder.machine.lock().unwrap();
    if !machine.is_idle() {
        return Err(state::TransitionError::AlreadyRecording.to_string());
    }
    let now = chrono::Local::now();
    let path = output_dir(&app).and_then(|dir| {
        let path = paths::unique_path(&dir, &paths::file_stem(now.naive_local()), Path::exists);
        Session::start(&path).map(|session| (path, session))
    });
    match path {
        Ok((path, session)) => {
            *recorder.session.lock().unwrap() = Some(session);
            machine.start(path, now.timestamp_millis()).map_err(|e| e.to_string())?;
            Ok(publish(&app, machine.status()))
        }
        Err(error) => {
            machine.fail(error.clone());
            publish(&app, machine.status());
            Err(error)
        }
    }
}

#[tauri::command]
pub async fn stop_recording(app: AppHandle, recorder: State<'_, Recorder>) -> Result<Status, String> {
    let session = {
        let mut machine = recorder.machine.lock().unwrap();
        machine.begin_stop().map_err(|e| e.to_string())?;
        publish(&app, machine.status());
        recorder.session.lock().unwrap().take()
    };
    let outcome = match session {
        Some(session) => tauri::async_runtime::spawn_blocking(move || session.stop())
            .await
            .unwrap_or_else(|e| Err(e.to_string())),
        None => Err("Sessão de captura perdida.".into()),
    };
    let mut machine = recorder.machine.lock().unwrap();
    machine.finish(outcome.clone());
    let status = publish(&app, machine.status());
    outcome.map(|()| status)
}

#[tauri::command]
pub fn open_output_dir(app: AppHandle) -> Result<(), String> {
    let dir = output_dir(&app)?;
    app.opener().open_path(dir.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}
