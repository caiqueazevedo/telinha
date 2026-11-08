import React from "react";
import ReactDOM from "react-dom/client";
import { RegionPicker } from "./RegionPicker";
import "./region.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <RegionPicker scale={window.devicePixelRatio} />
  </React.StrictMode>,
);
