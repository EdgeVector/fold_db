import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "path";

// https://vitejs.dev/config/
export default defineConfig({
  base: "./",
  plugins: [react()],
  resolve: {
    alias: {
      // Import TypeScript bindings directly from where ts-rs generates them
      "@generated": path.resolve(
        __dirname,
        "../../../bindings/src/fold_node/static-react/src/types",
      ),
    },
  },
  server: {
    proxy: {
      "/api": {
        target: "http://localhost:9001",
        changeOrigin: true,
        secure: false,
      },
    },
    watch: {
      ignored: ["**/node_modules/**"],
    },
  },
  build: {
    minify: false,
    sourcemap: true,
  },
});
