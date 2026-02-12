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
  build: {
    outDir: "dist-lib",
    lib: {
      entry: path.resolve(__dirname, "src/index.js"),
      name: "FoldNodeUI",
      fileName: (format) => `fold-node-ui.${format}.js`,
    },
    rollupOptions: {
      // make sure to externalize deps that shouldn't be bundled
      // into your library - especially React and Redux to prevent
      // multiple instances when consumed by an app
      external: [
        "react",
        "react-dom",
        "react-redux",
        "react/jsx-runtime",
        "react/jsx-dev-runtime",
        "@reduxjs/toolkit",
      ],
      output: {
        // Provide global variables to use in the UMD build
        // for externalized deps
        globals: {
          react: "React",
          "react-dom": "ReactDOM",
          "react-redux": "ReactRedux",
          "react/jsx-runtime": "ReactJSXRuntime",
          "react/jsx-dev-runtime": "ReactJSXDevRuntime",
          "@reduxjs/toolkit": "RTK",
        },
      },
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
});
