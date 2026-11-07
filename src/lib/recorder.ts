import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";

type Phase =
  | { phase: "idle" }
  | { phase: "recording"; startedAtMs: number; path: string }
  | { phase: "stopping"; path: string };

export type RecordingStatus = Phase & {
  lastRecording: string | null;
  error: string | null;
};

export const STATE_EVENT = "recording://state";

export const recorder = {
  status: () => invoke<RecordingStatus>("recording_status"),
  start: () => invoke<RecordingStatus>("start_recording"),
  stop: () => invoke<RecordingStatus>("stop_recording"),
  openOutputDir: () => invoke<void>("open_output_dir"),
  onState: (handler: (status: RecordingStatus) => void): Promise<UnlistenFn> =>
    listen<RecordingStatus>(STATE_EVENT, (event) => handler(event.payload)),
};
