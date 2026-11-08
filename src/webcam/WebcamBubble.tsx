import { useEffect, useRef, useState, type MouseEvent } from "react";

type Props = {
  /** Starts a native window drag; called on a left press anywhere but the close button. */
  onDrag: () => void;
  onClose: () => void;
};

type Camera = { state: "loading" } | { state: "live" } | { state: "error"; message: string };

function describe(error: unknown): string {
  const name = error instanceof DOMException ? error.name : "";
  if (name === "NotAllowedError") return "Câmera bloqueada";
  if (name === "NotFoundError") return "Nenhuma câmera";
  return "Câmera indisponível";
}

export function WebcamBubble({ onDrag, onClose }: Props) {
  const video = useRef<HTMLVideoElement>(null);
  const [camera, setCamera] = useState<Camera>({ state: "loading" });

  useEffect(() => {
    let cancelled = false;
    let stream: MediaStream | null = null;
    navigator.mediaDevices
      .getUserMedia({ video: { width: 640, height: 640, facingMode: "user" }, audio: false })
      .then((next) => {
        stream = next;
        if (cancelled) return next.getTracks().forEach((track) => track.stop());
        if (video.current) video.current.srcObject = next;
        setCamera({ state: "live" });
      })
      .catch((error) => {
        if (!cancelled) setCamera({ state: "error", message: describe(error) });
      });
    return () => {
      cancelled = true;
      stream?.getTracks().forEach((track) => track.stop());
    };
  }, []);

  function press(event: MouseEvent) {
    if (event.button === 0) onDrag();
  }

  return (
    <div className="bubble" onMouseDown={press} aria-label="Webcam">
      <video ref={video} autoPlay muted playsInline hidden={camera.state !== "live"} />
      {camera.state === "error" && <p role="alert">{camera.message}</p>}
      <button
        type="button"
        className="close"
        aria-label="Fechar webcam"
        onMouseDown={(event) => event.stopPropagation()}
        onClick={onClose}
      >
        ×
      </button>
    </div>
  );
}
