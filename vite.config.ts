import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";
import { tanstackRouter } from "@tanstack/router-plugin/vite";

// @ts-expect-error process is a nodejs global
const host = process.env.TAURI_DEV_HOST;

export default defineConfig(async () => ({
  plugins: [
    tanstackRouter({
      routesDirectory: "src/routes",
      generatedRouteTree: "src/routeTree.gen.ts",
    }),
    react(),
    tailwindcss(),
  ],

  clearScreen: false,

  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? { protocol: "ws", host, port: 1421 }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },

  resolve: {
    alias: {
      "@": "/src",
    },
  },

  build: {
    // Target modern Chromium/WebKit shipped with Tauri
    target: ["es2021", "chrome100", "safari15"],
    minify: "esbuild",
    rollupOptions: {
      output: {
        manualChunks: {
          // Router + data-fetching layer (React co-bundled here)
          "vendor-router": [
            "@tanstack/react-router",
            "@tanstack/react-query",
          ],
          // Tauri API surface — changes with Tauri upgrades, not app code
          "vendor-tauri": [
            "@tauri-apps/api",
            "@tauri-apps/plugin-shell",
            "@tauri-apps/plugin-dialog",
            "@tauri-apps/plugin-opener",
          ],
          // UI primitives — large, stable
          "vendor-ui": [
            "radix-ui",
            "lucide-react",
            "clsx",
            "class-variance-authority",
            "tailwind-merge",
            "sonner",
          ],
        },
      },
    },
  },
}));
