/**
 * Testing utilities for Tauri RPC
 * 
 * Provides mocking helpers for unit and integration tests.
 */

import { vi } from 'vitest';
import type { RouterDef, ProcedureDef } from './types';

type MockedProcedures<T extends RouterDef> = {
  [K in keyof T]: T[K] extends ProcedureDef<infer I, infer O>
    ? ReturnType<typeof vi.fn<[I], Promise<O>>>
    : T[K] extends RouterDef
      ? MockedProcedures<T[K]>
      : never;
};

interface MockClientOptions {
  defaultDelay?: number;
}

/**
 * Create a mock client for testing
 * 
 * @example
 * ```ts
 * const { mocks, mockInvoke } = createMockClient(appRouter);
 * 
 * // Setup mock response
 * mocks.greet.mockResolvedValue('Hello, Test!');
 * 
 * // The mockInvoke will route to the correct mock based on command
 * ```
 */
export function createMockClient<T extends RouterDef>(
  routerDef: T,
  options: MockClientOptions = {}
) {
  const { defaultDelay = 0 } = options;
  const mocks: Record<string, ReturnType<typeof vi.fn>> = {};
  const commandToMock: Record<string, ReturnType<typeof vi.fn>> = {};

  // Recursively create mocks for all procedures
  function createMocks(router: RouterDef, prefix = ''): any {
    const result: Record<string, any> = {};

    for (const [key, value] of Object.entries(router)) {
      const path = prefix ? `${prefix}.${key}` : key;

      if ('_command' in value) {
        const proc = value as ProcedureDef<any, any>;
        const mock = vi.fn();
        mocks[path] = mock;
        commandToMock[proc._command] = mock;
        result[key] = mock;
      } else {
        result[key] = createMocks(value as RouterDef, path);
      }
    }

    return result;
  }

  const mockedProcedures = createMocks(routerDef) as MockedProcedures<T>;

  // Mock invoke function that routes to the correct mock
  const mockInvoke = vi.fn(async (command: string, args?: any) => {
    const mock = commandToMock[command];
    if (!mock) {
      throw new Error(`No mock found for command: ${command}`);
    }

    if (defaultDelay > 0) {
      await new Promise((r) => setTimeout(r, defaultDelay));
    }

    return mock(args);
  });

  return {
    mocks: mockedProcedures,
    mockInvoke,
    commandToMock,
    
    // Helper to reset all mocks
    resetAll: () => {
      Object.values(mocks).forEach((m) => m.mockReset());
      mockInvoke.mockClear();
    },
  };
}

/**
 * Create a mock response helper with common patterns
 */
export function mockResponse<T>(data: T) {
  return {
    success: () => Promise.resolve(data),
    error: (message: string) => Promise.reject(new Error(message)),
    delay: (ms: number) => new Promise<T>((r) => setTimeout(() => r(data), ms)),
    sequence: (responses: T[]) => {
      let index = 0;
      return () => Promise.resolve(responses[index++ % responses.length]);
    },
  };
}

/**
 * Wait for all pending promises to resolve
 */
export function flushPromises() {
  return new Promise((resolve) => setTimeout(resolve, 0));
}

/**
 * Create a deferred promise for testing async flows
 */
export function createDeferred<T>() {
  let resolve!: (value: T) => void;
  let reject!: (error: Error) => void;

  const promise = new Promise<T>((res, rej) => {
    resolve = res;
    reject = rej;
  });

  return { promise, resolve, reject };
}
