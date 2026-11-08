import { useEffect, useState, type MouseEvent } from "react";
import { isUsable, normalizeDrag, type Point } from "../lib/geometry";
import { recorder } from "../lib/recorder";

type Props = { scale: number };

function pointOf(event: MouseEvent): Point {
  return { x: event.clientX, y: event.clientY };
}

export function RegionPicker({ scale }: Props) {
  const [start, setStart] = useState<Point | null>(null);
  const [end, setEnd] = useState<Point | null>(null);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      if (event.key === "Escape") void recorder.finishRegionPick(null);
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, []);

  function release(event: MouseEvent) {
    if (!start) return;
    const selection = normalizeDrag(start, pointOf(event), scale);
    setStart(null);
    setEnd(null);
    if (isUsable(selection)) void recorder.finishRegionPick(selection);
  }

  const box = start && end ? normalizeDrag(start, end) : null;
  const physical = start && end ? normalizeDrag(start, end, scale) : null;

  return (
    <div
      className="picker"
      data-testid="picker"
      onMouseDown={(event) => {
        setStart(pointOf(event));
        setEnd(pointOf(event));
      }}
      onMouseMove={(event) => start && setEnd(pointOf(event))}
      onMouseUp={release}
    >
      {box && physical ? (
        <div
          className="selection"
          aria-label="Área selecionada"
          style={{ left: box.x, top: box.y, width: box.width, height: box.height }}
        >
          <span className="size">
            {physical.width} × {physical.height}
          </span>
        </div>
      ) : (
        <p className="hint">Arraste para selecionar a área. Esc cancela.</p>
      )}
    </div>
  );
}
