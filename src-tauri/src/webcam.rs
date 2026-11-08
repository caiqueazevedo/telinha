//! The floating webcam bubble. It sits on screen, so monitor and region captures record it.

use tauri::webview::{PermissionKind, PermissionResponse};
use tauri::{AppHandle, Emitter, Manager, PhysicalPosition, PhysicalSize, WebviewUrl, WebviewWindowBuilder};

pub const WEBCAM_LABEL: &str = "webcam";
pub const CLOSED_EVENT: &str = "webcam://closed";

/// Bubble diameter in logical pixels.
const DIAMETER: f64 = 200.0;
/// Gap between the bubble and the work area edge, in logical pixels.
const MARGIN: f64 = 24.0;

/// The bubble loads only our own page, so its camera request needs no WebView2 prompt.
fn permission(kind: PermissionKind) -> PermissionResponse {
    match kind {
        PermissionKind::Camera => PermissionResponse::Allow,
        _ => PermissionResponse::Deny,
    }
}

/// Bottom-right corner of the work area, so the bubble clears the taskbar.
fn corner(area_pos: (i32, i32), area_size: (u32, u32), side: u32, margin: u32) -> (i32, i32) {
    let x = area_pos.0 + area_size.0 as i32 - side as i32 - margin as i32;
    let y = area_pos.1 + area_size.1 as i32 - side as i32 - margin as i32;
    (x.max(area_pos.0), y.max(area_pos.1))
}

// Async so the window is built off the main thread; sync commands deadlock on Windows here.
#[tauri::command]
pub async fn open_webcam(app: AppHandle) -> Result<(), String> {
    if let Some(open) = app.get_webview_window(WEBCAM_LABEL) {
        return open.set_focus().map_err(|e| e.to_string());
    }
    let window = WebviewWindowBuilder::new(&app, WEBCAM_LABEL, WebviewUrl::App("webcam.html".into()))
        .title("Webcam")
        .inner_size(DIAMETER, DIAMETER)
        .transparent(true)
        .decorations(false)
        .shadow(false)
        .resizable(false)
        .always_on_top(true)
        .skip_taskbar(true)
        .visible(false)
        .on_permission_request(|_, kind| permission(kind))
        .build()
        .map_err(|e| e.to_string())?;
    if let Some(monitor) = app.primary_monitor().map_err(|e| e.to_string())? {
        let scale = monitor.scale_factor();
        let side = (DIAMETER * scale).round() as u32;
        let area = monitor.work_area();
        let (x, y) = corner(
            (area.position.x, area.position.y),
            (area.size.width, area.size.height),
            side,
            (MARGIN * scale).round() as u32,
        );
        window.set_size(PhysicalSize::new(side, side)).map_err(|e| e.to_string())?;
        window.set_position(PhysicalPosition::new(x, y)).map_err(|e| e.to_string())?;
    }
    window.show().map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn close_webcam(app: AppHandle) -> Result<(), String> {
    if let Some(window) = app.get_webview_window(WEBCAM_LABEL) {
        window.destroy().map_err(|e| e.to_string())?;
    }
    app.emit_to("main", CLOSED_EVENT, ()).map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_the_camera_is_granted() {
        assert!(matches!(permission(PermissionKind::Camera), PermissionResponse::Allow));
        assert!(matches!(permission(PermissionKind::Microphone), PermissionResponse::Deny));
    }

    #[test]
    fn corner_sits_inside_the_work_area_with_a_margin() {
        assert_eq!(corner((0, 0), (1920, 1040), 200, 24), (1696, 816));
    }

    #[test]
    fn corner_follows_a_secondary_monitor_offset() {
        assert_eq!(corner((-1280, 100), (1280, 984), 300, 36), (-336, 748));
    }

    #[test]
    fn corner_never_leaves_a_tiny_work_area() {
        assert_eq!(corner((10, 20), (100, 100), 200, 24), (10, 20));
    }
}
