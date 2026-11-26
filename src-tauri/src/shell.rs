use tauri::{AppHandle, Emitter, WebviewWindow};
use tauri_plugin_global_shortcut::{GlobalShortcutExt, ShortcutState};
use windows::Win32::Foundation::HWND;
use windows::Win32::UI::WindowsAndMessaging::{SetWindowDisplayAffinity, WDA_EXCLUDEFROMCAPTURE};

pub const TOGGLE_SHORTCUT: &str = "ctrl+shift+r";
pub const TOGGLE_EVENT: &str = "shortcut://toggle-recording";

pub fn shortcut_plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    tauri_plugin_global_shortcut::Builder::new()
        .with_handler(|app, _shortcut, event| {
            if event.state == ShortcutState::Pressed {
                let _ = app.emit_to("main", TOGGLE_EVENT, ());
            }
        })
        .build()
}

pub fn register_shortcut(app: &AppHandle) {
    if let Err(error) = app.global_shortcut().register(TOGGLE_SHORTCUT) {
        eprintln!("telinha: atalho {TOGGLE_SHORTCUT} indisponível: {error}");
    }
}

pub fn exclude_from_capture(window: &WebviewWindow) {
    let result = window
        .hwnd()
        .map_err(|e| e.to_string())
        .and_then(|hwnd| unsafe { SetWindowDisplayAffinity(HWND(hwnd.0), WDA_EXCLUDEFROMCAPTURE) }.map_err(|e| e.to_string()));
    if let Err(error) = result {
        eprintln!("telinha: não foi possível ocultar o painel da captura: {error}");
    }
}
