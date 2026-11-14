import React from "react";
import { createRoot } from "react-dom/client";
import { App } from "@/app";
import "react-grid-layout/css/styles.css";
import "@/index.css";

const root = document.getElementById("root");

if (!root) {
  throw new Error("Vesta dashboard root was not found");
}

createRoot(root).render(
  <React.StrictMode>
    <App />
  </React.StrictMode>,
);
