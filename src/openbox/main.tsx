import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import { OpenBoxApp } from "./OpenBoxApp";
import "./assets/fonts/MiSans-VF-tRsyHePl.css";
import "./openbox.css";

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <OpenBoxApp />
  </StrictMode>,
);
