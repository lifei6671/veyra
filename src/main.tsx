import { StrictMode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

// 只取消 WebView 默认菜单，继续传递事件以保留应用自定义菜单。
document.addEventListener("contextmenu", event => event.preventDefault(), { capture: true });

createRoot(document.getElementById("root")!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
