import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

const openBoxProxy = {
  "/api": {
    target: "https://openbox.disign.me",
    changeOrigin: true,
    ws: true,
  },
};

export default defineConfig({
  plugins: [react()],
  resolve: {
    dedupe: ["react", "react-dom"],
  },
  build: {
    rollupOptions: {
      input: {
        main: "index.html",
        openbox: "openbox.html",
      },
    },
  },
  server: {
    watch: { ignored: ["**/src-tauri/target/**", "**/.sdlc/evidence/**"] },
    proxy: openBoxProxy,
  },
  preview: {
    proxy: openBoxProxy,
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts", "scripts/**/*.test.mjs"],
  },
});
