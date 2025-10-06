import vue from "@vitejs/plugin-vue";
import { defineConfig } from "vite";

// https://vite.dev/config/
export default defineConfig({
  plugins: [vue()],
  build: {
    rollupOptions: {
      output: {
        entryFileNames: 'assets/index.js',
        manualChunks: undefined,
      }
    },
    cssCodeSplit: false,
    assetsInlineLimit: 99999999999
  }
});
