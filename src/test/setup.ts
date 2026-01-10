import { vi } from 'vitest';
import '@testing-library/jest-dom/vitest';

// Mock Tauri's invoke function globally
vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));
