import { vi } from "vitest";
import "@testing-library/jest-dom/vitest";

// Mock Tauri's invoke function globally
vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
}));

// Mock Tauri's event API globally
vi.mock("@tauri-apps/api/event", () => ({
  listen: vi.fn().mockResolvedValue(vi.fn()),
  emit: vi.fn(),
}));
