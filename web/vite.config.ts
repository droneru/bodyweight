import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

// In `make dev` the axum server listens on :3000; the Vite dev server proxies the API to it.
export default defineConfig({
  plugins: [react()],
  server: {
    host: true,
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": "http://127.0.0.1:3000",
    },
  },
});
