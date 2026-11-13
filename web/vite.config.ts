import { fileURLToPath } from "node:url";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

// In `make dev` the axum server listens on :3000; the Vite dev server proxies the API to it.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  resolve: {
    // The alias shadcn components are imported by, see components.json.
    alias: { "@": fileURLToPath(new URL("./src", import.meta.url)) },
  },
  server: {
    host: true,
    port: 5173,
    strictPort: true,
    proxy: {
      "/api": "http://127.0.0.1:3000",
    },
  },
});
