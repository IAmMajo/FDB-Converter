import { defineConfig } from "vite";

export default defineConfig({
  build: {
    rollupOptions: {
      input: [
        "index.html",
        "sqlite-to-fdb/index.html",
        "xml-to-fdb/index.html",
      ],
    },
  },
});
