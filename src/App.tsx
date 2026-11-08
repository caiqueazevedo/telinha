import { useEffect, useRef, useState } from "react";
import { formatElapsed } from "./lib/format";
import { AudioSource } from "./components/AudioSource";
import { Recents } from "./components/Recents";
import { CaptureSource, targetOf, type CaptureChoice } from "./components/CaptureSource";
import {
  recorder,
  type AudioDevice,
  type AudioDevices,
  type Recording,
  type RecordingStatus,
  type Sources,
} from "./lib/recorder";
import "./App.css";

function useNow(active: boolean): number {
  const [now, setNow] = useState(() => Date.now());
  useEffect(() => {
    if (!active) return;
    setNow(Date.now());
    const id = setInterval(() => setNow(Date.now()), 250);
    return () => clearInterval(id);
  }, [active]);
  return now;
}

type SourceChoice = { enabled: boolean; deviceId: string | null };

function defaultChoice(devices: AudioDevice[]): SourceChoice {
  const device = devices.find((d) => d.isDefault) ?? devices[0];
  return { enabled: Boolean(device), deviceId: device?.id ?? null };
}

function selected(choice: SourceChoice): string | null {
  return choice.enabled ? choice.deviceId : null;
}

function App() {
  const [status, setStatus] = useState<RecordingStatus | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);
  const [devices, setDevices] = useState<AudioDevices>({ inputs: [], outputs: [] });
  const [mic, setMic] = useState<SourceChoice>({ enabled: false, deviceId: null });
  const [system, setSystem] = useState<SourceChoice>({ enabled: false, deviceId: null });
  const [sources, setSources] = useState<Sources>({ monitors: [], windows: [] });
  const [webcam, setWebcam] = useState(false);
  const [recents, setRecents] = useState<Recording[]>([]);
  const [capture, setCapture] = useState<CaptureChoice>({ mode: "monitor", monitor: null, window: null, region: null });

  function loadSources() {
    recorder
      .sources()
      .then((next) => {
        setSources(next);
        setCapture((current) => {
          const primary = next.monitors.find((m) => m.primary) ?? next.monitors[0];
          const monitor = next.monitors.some((m) => m.id === current.monitor) ? current.monitor : (primary?.id ?? null);
          const window = next.windows.some((w) => w.id === current.window) ? current.window : null;
          return { ...current, monitor, window, region: monitor === current.monitor ? current.region : null };
        });
      })
      .catch((error) => setCommandError(String(error)));
  }

  useEffect(loadSources, []);

  useEffect(() => {
    const unlisten = recorder.onRegionSelected((region) => {
      // A cancelled pick keeps the previous area.
      if (region) setCapture((current) => ({ ...current, monitor: region.monitor, region }));
    });
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  useEffect(() => {
    const unlisten = recorder.onWebcamClosed(() => setWebcam(false));
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  async function toggleWebcam(enabled: boolean) {
    setCommandError(null);
    try {
      if (enabled) await recorder.openWebcam();
      else await recorder.closeWebcam();
      setWebcam(enabled);
    } catch (error) {
      setCommandError(String(error));
    }
  }

  useEffect(() => {
    recorder
      .audioDevices()
      .then((next) => {
        setDevices(next);
        setMic(defaultChoice(next.inputs));
        setSystem(defaultChoice(next.outputs));
      })
      .catch((error) => setCommandError(String(error)));
  }, []);

  useEffect(() => {
    let disposed = false;
    const unlisten = recorder.onState((next) => {
      if (!disposed) setStatus(next);
    });
    recorder.status().then((next) => {
      if (!disposed) setStatus(next);
    });
    return () => {
      disposed = true;
      unlisten.then((stop) => stop());
    };
  }, []);

  const recording = status?.phase === "recording";
  const stopping = status?.phase === "stopping";
  const now = useNow(recording);
  const target = targetOf(capture);

  async function toggle() {
    if (!status || stopping) return;
    setCommandError(null);
    try {
      if (recording) setStatus(await recorder.stop());
      else if (target) setStatus(await recorder.start(target, { mic: selected(mic), system: selected(system) }));
    } catch (error) {
      setCommandError(String(error));
    }
  }

  // The shortcut listener is registered once, so it calls whatever toggle is current.
  const toggleRef = useRef(toggle);
  toggleRef.current = toggle;
  useEffect(() => {
    const unlisten = recorder.onShortcut(() => void toggleRef.current());
    return () => {
      unlisten.then((stop) => stop());
    };
  }, []);

  const lastRecording = status?.lastRecording ?? null;
  useEffect(() => {
    recorder
      .recordings()
      .then(setRecents)
      .catch((error) => setCommandError(String(error)));
  }, [lastRecording]);

  function openRecording(name: string) {
    recorder.openRecording(name).catch((error) => setCommandError(String(error)));
  }

  const error = commandError ?? status?.error;

  return (
    <main className="panel">
      <div className="timer" aria-label="Tempo de gravação">
        {status?.phase === "recording" ? formatElapsed(now - status.startedAtMs) : "00:00"}
      </div>

      <button
        type="button"
        className={recording ? "record recording" : "record"}
        onClick={toggle}
        disabled={!status || stopping || (!recording && !target)}
      >
        {stopping ? "Finalizando…" : recording ? "Parar" : "Gravar"}
      </button>
      <span className="hint">Ctrl+Shift+R</span>

      <fieldset className="sources" disabled={!status || recording || stopping}>
        <legend>Captura</legend>
        <CaptureSource
          sources={sources}
          choice={capture}
          onChange={setCapture}
          onRefresh={loadSources}
          onPickRegion={(monitor) => recorder.openRegionPicker(monitor).catch((error) => setCommandError(String(error)))}
        />
      </fieldset>

      <fieldset className="sources" disabled={!status || recording || stopping}>
        <legend>Áudio</legend>
        <AudioSource
          label="Microfone"
          devices={devices.inputs}
          enabled={mic.enabled}
          deviceId={mic.deviceId}
          onEnabledChange={(enabled) => setMic({ ...mic, enabled })}
          onDeviceChange={(deviceId) => setMic({ ...mic, deviceId })}
        />
        <AudioSource
          label="Sistema"
          devices={devices.outputs}
          enabled={system.enabled}
          deviceId={system.deviceId}
          onEnabledChange={(enabled) => setSystem({ ...system, enabled })}
          onDeviceChange={(deviceId) => setSystem({ ...system, deviceId })}
        />
      </fieldset>

      {/* The bubble can be toggled mid-recording; the window capture mode never shows it. */}
      <label className="toggle">
        <input type="checkbox" checked={webcam} onChange={(event) => toggleWebcam(event.target.checked)} />
        Webcam
      </label>

      {error && (
        <p className="error" role="alert">
          {error}
        </p>
      )}

      {status?.lastRecording && (
        <p className="last" title={status.lastRecording}>
          Salvo: {status.lastRecording.split(/[\\/]/).pop()}
        </p>
      )}

      <Recents recordings={recents} onOpen={openRecording} />

      <button type="button" className="link" onClick={() => recorder.openOutputDir()}>
        Abrir pasta
      </button>
    </main>
  );
}

export default App;
