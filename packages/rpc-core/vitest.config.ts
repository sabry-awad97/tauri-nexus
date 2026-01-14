import { defineConfig } from "vitest/config";
import { resolve } from "path";

export default defineConfig({
  test: {
    root: resolve(__dirname),
    globals: true,
    environment: "node",
    include: ["src/**/*.{test,spec}.{ts,tsx}"],
  },
});
