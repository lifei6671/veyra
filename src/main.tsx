import { StrictMode, type ReactNode } from "react";
import { createRoot } from "react-dom/client";
import App from "./App";
import "./styles.css";

// 只取消 WebView 默认菜单，继续传递事件以保留应用自定义菜单。
document.addEventListener("contextmenu", event => event.preventDefault(), { capture: true });

const root = createRoot(document.getElementById("root")!);
const render = (content: ReactNode) => root.render(<StrictMode>{content}</StrictMode>);

if (import.meta.env.DEV && new URLSearchParams(window.location.search).get("visual-harness") === "settings") {
  void import("./dev/SettingsVisualHarness").then(({ SettingsVisualHarness }) => render(<SettingsVisualHarness />));
} else {
  render(<App />);
}
