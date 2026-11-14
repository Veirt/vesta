import path from "node:path";
import tailwindcss from "@tailwindcss/vite";
import react from "@vitejs/plugin-react";
import { defineConfig } from "vite";

const frontendRoot = path.resolve(import.meta.dirname, "frontend");

export default defineConfig({
  root: frontendRoot,
  plugins: [react(), tailwindcss()],
  resolve: {
    alias: {
      "@": path.resolve(frontendRoot, "src"),
    },
  },
  server: {
    host: "127.0.0.1",
    port: 5173,
    proxy: {
      "/api": "http://127.0.0.1:3000",
      "/static": "http://127.0.0.1:3000",
    },
  },
  build: {
    outDir: "../static/ui",
    emptyOutDir: true,
    cssCodeSplit: false,
    rollupOptions: {
      output: {
        entryFileNames: "app.js",
        chunkFileNames: "assets/[name].js",
        assetFileNames: (asset) =>
          asset.name?.endsWith(".css") ? "app.css" : "assets/[name][extname]",
      },
    },
  },
});
