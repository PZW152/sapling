import React from "react";
import ReactDOM from "react-dom/client";
import "./shell/theme.css";
import "./shell/controls.css";
import "./shell/a11y.css";
import { Shell } from "./shell/Shell";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Shell />
  </React.StrictMode>,
);
