import { useEffect, useState } from "react";
import { formatElapsed } from "./lib/format";
import { recorder, type RecordingStatus } from "./lib/recorder";
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

function App() {
  const [status, setStatus] = useState<RecordingStatus | null>(null);
  const [commandError, setCommandError] = useState<string | null>(null);

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

  async function toggle() {
    setCommandError(null);
    try {
      setStatus(await (recording ? recorder.stop() : recorder.start()));
    } catch (error) {
      setCommandError(String(error));
    }
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
        disabled={!status || stopping}
      >
        {stopping ? "Finalizando…" : recording ? "Parar" : "Gravar"}
      </button>

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

      <button type="button" className="link" onClick={() => recorder.openOutputDir()}>
        Abrir pasta
      </button>
    </main>
  );
}

export default App;
