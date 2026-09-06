import { defineConfig } from "vitest/config";
import react from "@vitejs/plugin-react";

export default defineConfig({
  plugins: [react()],
  server: {
    watch: { ignored: ["**/src-tauri/target/**", "**/.sdlc/evidence/**"] },
  },
  test: {
    environment: "node",
    include: ["src/**/*.test.ts"],
  },
});
