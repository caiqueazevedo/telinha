//! Desktop integration of the control panel: the global shortcut and capture exclusion.

use tauri::{AppHandle, Emitter, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};

pub const TOGGLE_SHORTCUT: &str = "ctrl+shift+r";
pub const TOGGLE_EVENT: &str = "shortcut://toggle-recording";

/// The panel decides what a toggle means, since it holds the chosen target and devices.
pub fn shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit_to("main", TOGGLE_EVENT, ());
            }
        })
        .build()
}

/// Another app may own the shortcut already; the panel button still works then.
pub fn register_shortcut(app: &AppHandle) {
    if let Err(error) = app.global_shortcut().register(TOGGLE_SHORTCUT) {
        eprintln!("telinha: atalho {TOGGLE_SHORTCUT} indisponível: {error}");
    }
}

/// Keeps the panel out of every capture (Windows 10 2004+); older systems just record it.
pub fn exclude_from_capture(window: &WebviewWindow) {
    let result = window
        .hwnd()
        .map_err(|e| e.to_string())
        .and_then(|hwnd| unsafe { SetWindowDisplayAffinity(HWND(hwnd.0), WDA_EXCLUDEFROMCAPTURE) }.map_err(|e| e.to_string()));
    if let Err(error) = result {
        eprintln!("telinha: não foi possível ocultar o painel da captura: {error}");
    }
}
