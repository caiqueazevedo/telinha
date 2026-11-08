import type { CaptureTarget, Region, Sources } from "../lib/recorder";

export type CaptureMode = "monitor" | "window" | "region";
export type CaptureChoice = {
  mode: CaptureMode;
  monitor: number | null;
  window: string | null;
  region: Region | null;
};

const MODES: { mode: CaptureMode; label: string }[] = [
  { mode: "monitor", label: "Tela" },
  { mode: "window", label: "Janela" },
  { mode: "region", label: "Região" },
];

/** The target to record, or `null` while the choice is incomplete. */
export function targetOf(choice: CaptureChoice): CaptureTarget | null {
  switch (choice.mode) {
    case "monitor":
      return choice.monitor === null ? null : { kind: "monitor", monitor: choice.monitor };
    case "window":
      return choice.window === null ? null : { kind: "window", id: choice.window };
    case "region":
      return choice.region === null ? null : { kind: "region", ...choice.region };
  }
}

type Props = {
  sources: Sources;
  choice: CaptureChoice;
  onChange: (choice: CaptureChoice) => void;
  onRefresh: () => void;
  onPickRegion: (monitor: number) => void;
};

export function CaptureSource({ sources, choice, onChange, onRefresh, onPickRegion }: Props) {
  return (
    <>
      <div className="modes" role="radiogroup" aria-label="Modo de captura">
        {MODES.map(({ mode, label }) => (
          <label key={mode} className={choice.mode === mode ? "mode active" : "mode"}>
            <input
              type="radio"
              name="capture-mode"
              checked={choice.mode === mode}
              onChange={() => onChange({ ...choice, mode })}
            />
            {label}
          </label>
        ))}
      </div>

      {choice.mode === "window" ? (
        <div className="source row">
          <select
            aria-label="Janela"
            value={choice.window ?? ""}
            onChange={(event) => onChange({ ...choice, window: event.target.value || null })}
          >
            <option value="">Escolha uma janela</option>
            {sources.windows.map((window) => (
              <option key={window.id} value={window.id}>
                {window.title}
              </option>
            ))}
          </select>
          <button type="button" className="small" onClick={onRefresh}>
            Atualizar
          </button>
        </div>
      ) : (
        <div className="source">
          <span>Monitor</span>
          <select
            aria-label="Monitor"
            value={choice.monitor ?? ""}
            onChange={(event) => onChange({ ...choice, monitor: Number(event.target.value), region: null })}
          >
            {sources.monitors.map((monitor) => (
              <option key={monitor.id} value={monitor.id}>
                {monitor.name} ({monitor.width}×{monitor.height})
              </option>
            ))}
          </select>
        </div>
      )}

      {choice.mode === "region" && (
        <div className="source">
          <button
            type="button"
            className="small"
            disabled={choice.monitor === null}
            onClick={() => choice.monitor !== null && onPickRegion(choice.monitor)}
          >
            Selecionar área
          </button>
          <span className="hint" aria-label="Área">
            {choice.region ? `${choice.region.width} × ${choice.region.height}` : "Nenhuma área"}
          </span>
        </div>
      )}
    </>
  );
}
