//! The full-monitor overlay where the user drags the capture region.

use std::sync::Mutex;

use serde::Serialize;
use tauri::{AppHandle, Emitter, Manager, State, WebviewUrl, WebviewWindowBuilder};

use crate::region::Selection;
use crate::sources;

pub const PICKER_LABEL: &str = "region-picker";
pub const SELECTED_EVENT: &str = "region://selected";

/// The monitor the open picker covers. The page only knows pixels relative to itself.
#[derive(Default)]
pub struct Picker(Mutex<Option<usize>>);

#[derive(Debug, Clone, Serialize)]
pub struct PickedRegion {
    pub monitor: usize,
    #[serde(flatten)]
    pub selection: Selection,
}

// Async so the window is built off the main thread; sync commands deadlock on Windows here.
#[tauri::command]
pub async fn open_region_picker(app: AppHandle, picker: State<'_, Picker>, monitor: usize) -> Result<(), String> {
    let source = sources::monitors()?
        .into_iter()
        .find(|m| m.id == monitor)
        .ok_or("Monitor não encontrado. Atualize a lista de fontes.")?;
    let target = app
        .available_monitors()
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|m| m.name().is_some_and(|name| *name == source.device_name))
        .ok_or("Monitor não encontrado pelo sistema de janelas.")?;
    if let Some(open) = app.get_webview_window(PICKER_LABEL) {
        open.destroy().map_err(|e| e.to_string())?;
    }
    *picker.0.lock().unwrap() = Some(monitor);
    let window = WebviewWindowBuilder::new(&app, PICKER_LABEL, WebviewUrl::App("region.html".into()))
        .title("Selecionar região")
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .build()
        .map_err(|e| e.to_string())?;
    window.set_position(*target.position()).map_err(|e| e.to_string())?;
    window.set_size(*target.size()).map_err(|e| e.to_string())?;
    window.show().map_err(|e| e.to_string())?;
    window.set_focus().map_err(|e| e.to_string())
}

/// Called by the picker page; `None` means the user cancelled.
#[tauri::command]
pub async fn finish_region_pick(
    app: AppHandle,
    picker: State<'_, Picker>,
    selection: Option<Selection>,
) -> Result<(), String> {
    let monitor = picker.0.lock().unwrap().take();
    let picked = monitor.zip(selection).map(|(monitor, selection)| PickedRegion { monitor, selection });
    app.emit_to("main", SELECTED_EVENT, picked).map_err(|e| e.to_string())?;
    if let Some(window) = app.get_webview_window(PICKER_LABEL) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn picked_region_serializes_flat_for_the_front_end() {
        let picked = PickedRegion { monitor: 2, selection: Selection { x: 1, y: 2, width: 30, height: 40 } };
        assert_eq!(
            serde_json::to_string(&picked).unwrap(),
            r#"{"monitor":2,"x":1,"y":2,"width":30,"height":40}"#
        );
    }
}
