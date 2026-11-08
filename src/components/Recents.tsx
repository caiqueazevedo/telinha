import { formatBytes } from "../lib/format";
import type { Recording } from "../lib/recorder";

type Props = { recordings: Recording[]; onOpen: (name: string) => void };

export function Recents({ recordings, onOpen }: Props) {
  if (recordings.length === 0) return null;
  return (
    <section className="recents" aria-label="Recentes">
      <h2>Recentes</h2>
      <ul>
        {recordings.map((recording) => (
          <li key={recording.name}>
            <button type="button" className="recent" onClick={() => onOpen(recording.name)}>
              <span className="recent-name">{recording.name}</span>
              <span className="hint">{formatBytes(recording.bytes)}</span>
            </button>
          </li>
        ))}
      </ul>
    </section>
  );
}
