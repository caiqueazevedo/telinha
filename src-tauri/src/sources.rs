//! Capture sources the user can pick: monitors and top-level windows.

use serde::Serialize;
use windows_capture::monitor::Monitor;
use windows_capture::window::Window;

use crate::region::MIN_SIDE;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MonitorSource {
    /// One-based position in the enumeration, as `Monitor::from_index` expects.
    pub id: usize,
    pub name: String,
    pub device_name: String,
    pub width: u32,
    pub height: u32,
    pub primary: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WindowSource {
    /// The HWND as a decimal string, so JS numbers never round it.
    pub id: String,
    pub title: String,
    pub process: String,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Serialize)]
pub struct Sources {
    pub monitors: Vec<MonitorSource>,
    pub windows: Vec<WindowSource>,
}

pub fn monitors() -> Result<Vec<MonitorSource>, String> {
    let primary = Monitor::primary().and_then(|m| m.device_name()).ok();
    let list = Monitor::enumerate().map_err(|e| format!("Falha ao listar monitores: {e}"))?;
    Ok(list
        .into_iter()
        .enumerate()
        .filter_map(|(position, monitor)| {
            let id = position + 1;
            let device_name = monitor.device_name().ok()?;
            Some(MonitorSource {
                id,
                name: monitor.name().unwrap_or_else(|_| format!("Tela {id}")),
                primary: primary.as_deref() == Some(device_name.as_str()),
                device_name,
                width: monitor.width().ok()?,
                height: monitor.height().ok()?,
            })
        })
        .collect())
}

pub fn windows() -> Result<Vec<WindowSource>, String> {
    let own = std::process::id();
    let list = Window::enumerate().map_err(|e| format!("Falha ao listar janelas: {e}"))?;
    Ok(list
        .into_iter()
        .filter_map(|window| {
            if window.process_id().ok()? == own {
                return None;
            }
            let title = window.title().ok().filter(|t| !t.trim().is_empty())?;
            let width = u32::try_from(window.width().ok()?).ok().filter(|w| *w >= MIN_SIDE)?;
            let height = u32::try_from(window.height().ok()?).ok().filter(|h| *h >= MIN_SIDE)?;
            Some(WindowSource {
                id: (window.as_raw_hwnd() as usize).to_string(),
                title,
                process: window.process_name().unwrap_or_default(),
                width,
                height,
            })
        })
        .collect())
}

#[tauri::command]
pub fn list_sources() -> Result<Sources, String> {
    Ok(Sources { monitors: monitors()?, windows: windows()? })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Needs a real desktop session.
    #[test]
    #[ignore]
    fn lists_at_least_the_primary_monitor() {
        let sources = list_sources().unwrap();
        assert!(sources.monitors.iter().any(|m| m.primary));
    }
}
