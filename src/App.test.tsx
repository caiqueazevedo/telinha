import { act, render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { AudioDevices, RecordingStatus } from "./lib/recorder";

type Handler = (event: { payload: unknown }) => void;

const invoke = vi.hoisted(() => vi.fn());
const listeners = vi.hoisted(() => new Map<string, Handler>());
vi.mock("@tauri-apps/api/core", () => ({ invoke }));
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn(async (name: string, handler: Handler) => {
    listeners.set(name, handler);
    return () => listeners.delete(name);
  }),
}));

const idle: RecordingStatus = { phase: "idle", lastRecording: null, error: null };
const recording: RecordingStatus = {
  phase: "recording",
  startedAtMs: Date.now(),
  path: "a.mp4",
  lastRecording: null,
  error: null,
};

const devices: AudioDevices = {
  inputs: [
    { id: "wasapi:mic-1", name: "Headset", isDefault: false },
    { id: "wasapi:mic-2", name: "Webcam Mic", isDefault: true },
  ],
  outputs: [{ id: "wasapi:out-1", name: "Speakers", isDefault: true }],
};

function respond(responses: Record<string, unknown>) {
  invoke.mockImplementation(async (command: string) => {
    const response = { list_audio_devices: devices, ...responses }[command];
    if (response instanceof Error) throw response.message;
    return response;
  });
}

beforeEach(() => {
  invoke.mockReset();
  listeners.clear();
});

describe("App", () => {
  it("starts a recording and shows the stop button", async () => {
    respond({ recording_status: idle, start_recording: recording });
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: "Gravar" }));

    expect(invoke).toHaveBeenCalledWith("start_recording", {
      audio: { mic: "wasapi:mic-2", system: "wasapi:out-1" },
    });
    expect(await screen.findByRole("button", { name: "Parar" })).toBeEnabled();
  });

  it("records only the chosen sources and devices", async () => {
    respond({ recording_status: idle, start_recording: recording });
    render(<App />);
    const micDevice = screen.getByRole("combobox", { name: "Dispositivo de microfone" });
    await screen.findByRole("option", { name: "Headset" });

    await userEvent.selectOptions(micDevice, "wasapi:mic-1");
    await userEvent.click(screen.getByRole("checkbox", { name: "Sistema" }));
    await userEvent.click(await screen.findByRole("button", { name: "Gravar" }));

    expect(invoke).toHaveBeenCalledWith("start_recording", { audio: { mic: "wasapi:mic-1", system: null } });
  });

  it("locks the audio controls while recording", async () => {
    respond({ recording_status: recording });
    render(<App />);

    await screen.findByRole("button", { name: "Parar" });

    expect(screen.getByRole("checkbox", { name: "Microfone" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Dispositivo de microfone" })).toBeDisabled();
  });

  it("shows the saved file name after stopping", async () => {
    const done: RecordingStatus = { ...idle, lastRecording: "C:\\Videos\\Telinha\\telinha-1.mp4" };
    respond({ recording_status: recording, stop_recording: done });
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: "Parar" }));

    expect(await screen.findByText("Salvo: telinha-1.mp4")).toBeInTheDocument();
    expect(screen.getByRole("button", { name: "Gravar" })).toBeEnabled();
  });

  it("disables the button while the file is being finalized", async () => {
    respond({ recording_status: idle });
    render(<App />);
    await screen.findByRole("button", { name: "Gravar" });

    act(() =>
      listeners.get("recording://state")?.({
        payload: { phase: "stopping", path: "a.mp4", lastRecording: null, error: null },
      }),
    );

    expect(screen.getByRole("button", { name: "Finalizando…" })).toBeDisabled();
  });

  it("shows the backend error when starting fails", async () => {
    respond({ recording_status: idle, start_recording: new Error("Monitor principal indisponível") });
    render(<App />);

    await userEvent.click(await screen.findByRole("button", { name: "Gravar" }));

    expect(await screen.findByRole("alert")).toHaveTextContent("Monitor principal indisponível");
  });

  it("opens the output folder", async () => {
    respond({ recording_status: idle, open_output_dir: undefined });
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "Abrir pasta" }));

    expect(invoke).toHaveBeenCalledWith("open_output_dir");
  });
});
