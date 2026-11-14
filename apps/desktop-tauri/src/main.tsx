import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "./App";
import "./styles/medscale.css";

const root = document.getElementById("root");
if (!root) throw new Error("MedScale root element is missing");
createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
