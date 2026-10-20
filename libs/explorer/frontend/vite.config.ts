import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";

// The built interface is served by the Python package, from cfabric_browser/static.
// During development, `npm run dev` proxies the API to a running cfabric-browser.
export default defineConfig({
  plugins: [react(), tailwindcss()],
  build: {
    outDir: "../cfabric_browser/static",
    emptyOutDir: true,
    chunkSizeWarningLimit: 1500,
  },
  server: {
    port: 5173,
    proxy: {
      "/api": { target: "http://127.0.0.1:8107", changeOrigin: false },
    },
  },
});
