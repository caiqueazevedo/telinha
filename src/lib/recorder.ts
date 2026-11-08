import { invoke } from "@tauri-apps/api/core";
import { listen, type UnlistenFn } from "@tauri-apps/api/event";
import type { Rect } from "./geometry";

type Phase =
  | { phase: "idle" }
  | { phase: "recording"; startedAtMs: number; path: string }
  | { phase: "stopping"; path: string };

export type RecordingStatus = Phase & {
  lastRecording: string | null;
  error: string | null;
};

export type AudioDevice = { id: string; name: string; isDefault: boolean };
export type AudioDevices = { inputs: AudioDevice[]; outputs: AudioDevice[] };
/** Device ids to record; `null` leaves that source out. */
export type AudioOptions = { mic: string | null; system: string | null };

export type MonitorSource = {
  id: number;
  name: string;
  deviceName: string;
  width: number;
  height: number;
  primary: boolean;
};
export type WindowSource = { id: string; title: string; process: string; width: number; height: number };
export type Sources = { monitors: MonitorSource[]; windows: WindowSource[] };
/** A region in physical pixels, relative to the top-left corner of `monitor`. */
export type Region = Rect & { monitor: number };
export type CaptureTarget =
  | { kind: "monitor"; monitor: number }
  | { kind: "window"; id: string }
  | ({ kind: "region" } & Region);

export const STATE_EVENT = "recording://state";
export const REGION_EVENT = "region://selected";
export const WEBCAM_CLOSED_EVENT = "webcam://closed";

export const recorder = {
  status: () => invoke<RecordingStatus>("recording_status"),
  start: (target: CaptureTarget, audio: AudioOptions) =>
    invoke<RecordingStatus>("start_recording", { target, audio }),
  stop: () => invoke<RecordingStatus>("stop_recording"),
  audioDevices: () => invoke<AudioDevices>("list_audio_devices"),
  sources: () => invoke<Sources>("list_sources"),
  openOutputDir: () => invoke<void>("open_output_dir"),
  openRegionPicker: (monitor: number) => invoke<void>("open_region_picker", { monitor }),
  /** Sent by the picker window; `null` cancels. */
  finishRegionPick: (selection: Rect | null) => invoke<void>("finish_region_pick", { selection }),
  openWebcam: () => invoke<void>("open_webcam"),
  closeWebcam: () => invoke<void>("close_webcam"),
  onState: (handler: (status: RecordingStatus) => void): Promise<UnlistenFn> =>
    listen<RecordingStatus>(STATE_EVENT, (event) => handler(event.payload)),
  onRegionSelected: (handler: (region: Region | null) => void): Promise<UnlistenFn> =>
    listen<Region | null>(REGION_EVENT, (event) => handler(event.payload)),
  /** The bubble was closed, from the panel or from the bubble itself. */
  onWebcamClosed: (handler: () => void): Promise<UnlistenFn> => listen(WEBCAM_CLOSED_EVENT, () => handler()),
};
