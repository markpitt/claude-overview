import React from "react";
import ReactDOM from "react-dom/client";
import App from "./App";
import { startStore } from "./store";
import { startTheme } from "./lib/theme";
import "./styles.css";

startTheme();
void startStore();

ReactDOM.createRoot(document.getElementById("root") as HTMLElement).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
