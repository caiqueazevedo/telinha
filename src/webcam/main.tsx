import React from "react";
import ReactDOM from "react-dom/client";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { recorder } from "../lib/recorder";
import { WebcamBubble } from "./WebcamBubble";
import "./webcam.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <WebcamBubble onDrag={() => getCurrentWindow().startDragging()} onClose={() => recorder.closeWebcam()} />
  </React.StrictMode>,
);
