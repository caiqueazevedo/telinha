import { act, render, screen, waitFor } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { beforeEach, describe, expect, it, vi } from "vitest";
import App from "./App";
import type { AudioDevices, RecordingStatus, Sources } from "./lib/recorder";

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

const sources: Sources = {
  monitors: [
    { id: 1, name: "Monitor 1", deviceName: "\\\\.\\DISPLAY1", width: 1280, height: 1024, primary: false },
    { id: 2, name: "Monitor 2", deviceName: "\\\\.\\DISPLAY2", width: 1920, height: 1080, primary: true },
  ],
  windows: [{ id: "4242", title: "Editor", process: "code.exe", width: 800, height: 600 }],
};

const primary = { kind: "monitor", monitor: 2 };
const audio = { mic: "wasapi:mic-2", system: "wasapi:out-1" };

function respond(responses: Record<string, unknown>) {
  invoke.mockImplementation(async (command: string) => {
    const response = { list_audio_devices: devices, list_sources: sources, ...responses }[command];
    if (response instanceof Error) throw response.message;
    return response;
  });
}

async function recordButton() {
  const button = await screen.findByRole("button", { name: "Gravar" });
  await waitFor(() => expect(button).toBeEnabled());
  return button;
}

function emit(event: string, payload: unknown) {
  act(() => listeners.get(event)?.({ payload }));
}

beforeEach(() => {
  invoke.mockReset();
  listeners.clear();
});

describe("App", () => {
  it("records the primary monitor by default and shows the stop button", async () => {
    respond({ recording_status: idle, start_recording: recording });
    render(<App />);

    await userEvent.click(await recordButton());

    expect(invoke).toHaveBeenCalledWith("start_recording", { target: primary, audio });
    expect(await screen.findByRole("button", { name: "Parar" })).toBeEnabled();
  });

  it("records only the chosen sources and devices", async () => {
    respond({ recording_status: idle, start_recording: recording });
    render(<App />);
    const micDevice = screen.getByRole("combobox", { name: "Dispositivo de microfone" });
    await screen.findByRole("option", { name: "Headset" });

    await userEvent.selectOptions(micDevice, "wasapi:mic-1");
    await userEvent.click(screen.getByRole("checkbox", { name: "Sistema" }));
    await userEvent.click(await recordButton());

    expect(invoke).toHaveBeenCalledWith("start_recording", {
      target: primary,
      audio: { mic: "wasapi:mic-1", system: null },
    });
  });

  it("locks the capture and audio controls while recording", async () => {
    respond({ recording_status: recording });
    render(<App />);

    await screen.findByRole("button", { name: "Parar" });

    expect(screen.getByRole("checkbox", { name: "Microfone" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Dispositivo de microfone" })).toBeDisabled();
    expect(screen.getByRole("radio", { name: "Região" })).toBeDisabled();
    expect(screen.getByRole("combobox", { name: "Monitor" })).toBeDisabled();
  });

  it("records another monitor when chosen", async () => {
    respond({ recording_status: idle, start_recording: recording });
    render(<App />);
    await screen.findByRole("option", { name: /Monitor 1/ });

    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Monitor" }), "1");
    await userEvent.click(await recordButton());

    expect(invoke).toHaveBeenCalledWith("start_recording", { target: { kind: "monitor", monitor: 1 }, audio });
  });

  it("records the chosen window", async () => {
    respond({ recording_status: idle, start_recording: recording });
    render(<App />);
    await recordButton();

    await userEvent.click(screen.getByRole("radio", { name: "Janela" }));
    expect(screen.getByRole("button", { name: "Gravar" })).toBeDisabled();
    await userEvent.selectOptions(screen.getByRole("combobox", { name: "Janela" }), "4242");
    await userEvent.click(await recordButton());

    expect(invoke).toHaveBeenCalledWith("start_recording", { target: { kind: "window", id: "4242" }, audio });
  });

  it("refreshes the window list", async () => {
    respond({ recording_status: idle });
    render(<App />);
    await recordButton();
    await userEvent.click(screen.getByRole("radio", { name: "Janela" }));

    await userEvent.click(screen.getByRole("button", { name: "Atualizar" }));

    expect(invoke.mock.calls.filter(([command]) => command === "list_sources")).toHaveLength(2);
  });

  it("records the area picked on screen", async () => {
    respond({ recording_status: idle, start_recording: recording, open_region_picker: undefined });
    render(<App />);
    await recordButton();

    await userEvent.click(screen.getByRole("radio", { name: "Região" }));
    expect(screen.getByRole("button", { name: "Gravar" })).toBeDisabled();
    await userEvent.click(screen.getByRole("button", { name: "Selecionar área" }));
    expect(invoke).toHaveBeenCalledWith("open_region_picker", { monitor: 2 });

    emit("region://selected", { monitor: 2, x: 10, y: 20, width: 300, height: 200 });
    expect(screen.getByLabelText("Área")).toHaveTextContent("300 × 200");
    await userEvent.click(await recordButton());

    expect(invoke).toHaveBeenCalledWith("start_recording", {
      target: { kind: "region", monitor: 2, x: 10, y: 20, width: 300, height: 200 },
      audio,
    });
  });

  it("keeps the previous area when a pick is cancelled", async () => {
    respond({ recording_status: idle });
    render(<App />);
    await recordButton();
    await userEvent.click(screen.getByRole("radio", { name: "Região" }));

    emit("region://selected", { monitor: 2, x: 0, y: 0, width: 640, height: 360 });
    emit("region://selected", null);

    expect(screen.getByLabelText("Área")).toHaveTextContent("640 × 360");
  });

  it("opens and closes the webcam bubble", async () => {
    respond({ recording_status: idle, open_webcam: undefined, close_webcam: undefined });
    render(<App />);
    const webcam = screen.getByRole("checkbox", { name: "Webcam" });

    await userEvent.click(webcam);
    expect(invoke).toHaveBeenCalledWith("open_webcam");
    await waitFor(() => expect(webcam).toBeChecked());

    await userEvent.click(webcam);
    expect(invoke).toHaveBeenCalledWith("close_webcam");
    await waitFor(() => expect(webcam).not.toBeChecked());
  });

  it("keeps the webcam toggle usable while recording", async () => {
    respond({ recording_status: recording, open_webcam: undefined });
    render(<App />);
    await screen.findByRole("button", { name: "Parar" });

    await userEvent.click(screen.getByRole("checkbox", { name: "Webcam" }));

    expect(invoke).toHaveBeenCalledWith("open_webcam");
  });

  it("unchecks the webcam when the bubble closes itself", async () => {
    respond({ recording_status: idle, open_webcam: undefined });
    render(<App />);
    const webcam = screen.getByRole("checkbox", { name: "Webcam" });
    await userEvent.click(webcam);
    await waitFor(() => expect(webcam).toBeChecked());

    emit("webcam://closed", null);

    expect(webcam).not.toBeChecked();
  });

  it("leaves the webcam unchecked when it fails to open", async () => {
    respond({ recording_status: idle, open_webcam: new Error("Sem câmera") });
    render(<App />);
    const webcam = screen.getByRole("checkbox", { name: "Webcam" });

    await userEvent.click(webcam);

    expect(await screen.findByRole("alert")).toHaveTextContent("Sem câmera");
    expect(webcam).not.toBeChecked();
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

    emit("recording://state", { phase: "stopping", path: "a.mp4", lastRecording: null, error: null });

    expect(screen.getByRole("button", { name: "Finalizando…" })).toBeDisabled();
  });

  it("shows the backend error when starting fails", async () => {
    respond({ recording_status: idle, start_recording: new Error("Monitor principal indisponível") });
    render(<App />);

    await userEvent.click(await recordButton());

    expect(await screen.findByRole("alert")).toHaveTextContent("Monitor principal indisponível");
  });

  it("opens the output folder", async () => {
    respond({ recording_status: idle, open_output_dir: undefined });
    render(<App />);

    await userEvent.click(screen.getByRole("button", { name: "Abrir pasta" }));

    expect(invoke).toHaveBeenCalledWith("open_output_dir");
  });
});
