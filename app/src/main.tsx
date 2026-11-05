import React from "react";
import ReactDOM from "react-dom/client";
import { Shell } from "./shell/Shell";
import "./shell/theme.css";

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <Shell />
  </React.StrictMode>,
);
