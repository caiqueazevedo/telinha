use std::fmt;
use std::path::PathBuf;

use serde::Serialize;

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "phase", rename_all = "camelCase", rename_all_fields = "camelCase")]
pub enum Phase {
    Idle,
    Recording { started_at_ms: i64, path: PathBuf },
    Stopping { path: PathBuf },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    #[serde(flatten)]
    pub phase: Phase,
    pub last_recording: Option<PathBuf>,
    pub error: Option<String>,
}

#[derive(Debug, PartialEq)]
pub enum TransitionError {
    AlreadyRecording,
    NotRecording,
}

impl fmt::Display for TransitionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::AlreadyRecording => "Já existe uma gravação em andamento.",
            Self::NotRecording => "Nenhuma gravação em andamento.",
        })
    }
}

/// Recording lifecycle: `Idle -> Recording -> Stopping -> Idle`.
pub struct Machine {
    phase: Phase,
    last_recording: Option<PathBuf>,
    error: Option<String>,
}

impl Default for Machine {
    fn default() -> Self {
        Self { phase: Phase::Idle, last_recording: None, error: None }
    }
}

impl Machine {
    pub fn status(&self) -> Status {
        Status { phase: self.phase.clone(), last_recording: self.last_recording.clone(), error: self.error.clone() }
    }

    pub fn is_idle(&self) -> bool {
        self.phase == Phase::Idle
    }

    pub fn start(&mut self, path: PathBuf, now_ms: i64) -> Result<(), TransitionError> {
        if !self.is_idle() {
            return Err(TransitionError::AlreadyRecording);
        }
        self.phase = Phase::Recording { started_at_ms: now_ms, path };
        self.error = None;
        Ok(())
    }

    pub fn begin_stop(&mut self) -> Result<(), TransitionError> {
        let Phase::Recording { path, .. } = &self.phase else {
            return Err(TransitionError::NotRecording);
        };
        self.phase = Phase::Stopping { path: path.clone() };
        Ok(())
    }

    /// Closes a stop that is in progress with the outcome of finalizing the file.
    pub fn finish(&mut self, outcome: Result<(), String>) {
        let Phase::Stopping { path } = std::mem::replace(&mut self.phase, Phase::Idle) else {
            return;
        };
        match outcome {
            Ok(()) => self.last_recording = Some(path),
            Err(error) => self.error = Some(error),
        }
    }

    /// Records a failure to start; the machine stays idle.
    pub fn fail(&mut self, error: String) {
        self.phase = Phase::Idle;
        self.error = Some(error);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn recording() -> Machine {
        let mut machine = Machine::default();
        machine.start(PathBuf::from("a.mp4"), 1_000).unwrap();
        machine
    }

    #[test]
    fn start_from_idle_enters_recording() {
        let machine = recording();
        assert_eq!(machine.status().phase, Phase::Recording { started_at_ms: 1_000, path: "a.mp4".into() });
    }

    #[test]
    fn double_start_is_rejected() {
        let mut machine = recording();
        assert_eq!(machine.start("b.mp4".into(), 2_000), Err(TransitionError::AlreadyRecording));
    }

    #[test]
    fn stop_without_recording_is_rejected() {
        assert_eq!(Machine::default().begin_stop(), Err(TransitionError::NotRecording));
    }

    #[test]
    fn successful_stop_keeps_last_recording() {
        let mut machine = recording();
        machine.begin_stop().unwrap();
        assert_eq!(machine.status().phase, Phase::Stopping { path: "a.mp4".into() });
        machine.finish(Ok(()));
        let status = machine.status();
        assert_eq!(status.phase, Phase::Idle);
        assert_eq!(status.last_recording, Some("a.mp4".into()));
        assert_eq!(status.error, None);
    }

    #[test]
    fn failed_stop_reports_error_and_returns_to_idle() {
        let mut machine = recording();
        machine.begin_stop().unwrap();
        machine.finish(Err("boom".into()));
        let status = machine.status();
        assert_eq!(status.phase, Phase::Idle);
        assert_eq!(status.last_recording, None);
        assert_eq!(status.error.as_deref(), Some("boom"));
    }

    #[test]
    fn new_start_clears_previous_error() {
        let mut machine = Machine::default();
        machine.fail("no monitor".into());
        machine.start("a.mp4".into(), 0).unwrap();
        assert_eq!(machine.status().error, None);
    }

    #[test]
    fn status_serializes_with_camel_case_tag() {
        let json = serde_json::to_value(recording().status()).unwrap();
        assert_eq!(json["phase"], "recording");
        assert_eq!(json["startedAtMs"], 1_000);
        assert_eq!(json["path"], "a.mp4");
        assert!(json["lastRecording"].is_null());
    }
}
