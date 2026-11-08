//! The recent recordings shown under the record button.

use std::path::Path;
use std::time::UNIX_EPOCH;

use serde::Serialize;
use tauri::AppHandle;
use tauri_plugin_opener::OpenerExt;

use crate::recorder::output_dir;

const RECENT: usize = 5;

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Recording {
    pub name: String,
    pub bytes: u64,
    pub modified_ms: u64,
}

/// A file Telinha wrote: a bare name, so it can never point outside the output folder.
fn is_recording_name(name: &str) -> bool {
    name.starts_with("telinha-")
        && name.ends_with(".mp4")
        && !name.contains(['/', '\\', ':'])
        && !name.contains("..")
}

fn newest_first(mut recordings: Vec<Recording>, limit: usize) -> Vec<Recording> {
    recordings.retain(|r| is_recording_name(&r.name));
    recordings.sort_by(|a, b| b.modified_ms.cmp(&a.modified_ms).then_with(|| b.name.cmp(&a.name)));
    recordings.truncate(limit);
    recordings
}

fn scan(dir: &Path) -> Result<Vec<Recording>, String> {
    let entries = std::fs::read_dir(dir).map_err(|e| format!("Não foi possível ler {}: {e}", dir.display()))?;
    Ok(entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let meta = entry.metadata().ok().filter(|m| m.is_file())?;
            let modified = meta.modified().ok()?.duration_since(UNIX_EPOCH).ok()?;
            Some(Recording {
                name: entry.file_name().into_string().ok()?,
                bytes: meta.len(),
                modified_ms: modified.as_millis() as u64,
            })
        })
        .collect())
}

#[tauri::command]
pub fn list_recordings(app: AppHandle) -> Result<Vec<Recording>, String> {
    Ok(newest_first(scan(&output_dir(&app)?)?, RECENT))
}

#[tauri::command]
pub fn open_recording(app: AppHandle, name: String) -> Result<(), String> {
    if !is_recording_name(&name) {
        return Err("Gravação inválida.".into());
    }
    let path = output_dir(&app)?.join(&name);
    if !path.is_file() {
        return Err("A gravação não existe mais.".into());
    }
    app.opener().open_path(path.to_string_lossy(), None::<&str>).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rec(name: &str, modified_ms: u64) -> Recording {
        Recording { name: name.into(), bytes: 1, modified_ms }
    }

    #[test]
    fn accepts_only_bare_telinha_mp4_names() {
        assert!(is_recording_name("telinha-20260307-090503.mp4"));
        assert!(is_recording_name("telinha-20260307-090503-2.mp4"));
        assert!(!is_recording_name("notes.mp4"));
        assert!(!is_recording_name("telinha-1.mkv"));
        assert!(!is_recording_name(r"telinha-..\..\evil.mp4"));
        assert!(!is_recording_name("telinha-C:/x.mp4"));
    }

    #[test]
    fn lists_the_newest_recordings_first_up_to_the_limit() {
        let all = vec![rec("telinha-a.mp4", 10), rec("telinha-c.mp4", 30), rec("other.mp4", 99), rec("telinha-b.mp4", 20)];
        let names: Vec<_> = newest_first(all, 2).into_iter().map(|r| r.name).collect();
        assert_eq!(names, ["telinha-c.mp4", "telinha-b.mp4"]);
    }

    #[test]
    fn serializes_in_camel_case() {
        let json = serde_json::to_string(&rec("telinha-a.mp4", 5)).unwrap();
        assert_eq!(json, r#"{"name":"telinha-a.mp4","bytes":1,"modifiedMs":5}"#);
    }
}
