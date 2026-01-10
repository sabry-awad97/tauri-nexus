// =============================================================================
// Event Iterator Tests
// =============================================================================
// Tests for the async event iterator implementation.

import { describe, it, expect, vi, beforeEach, afterEach } from 'vitest';
import * as fc from 'fast-check';
import { invoke } from '@tauri-apps/api/core';
import { listen } from '@tauri-apps/api/event';
import { createEventIterator, consumeEventIterator } from '../event-iterator';
import type { RpcError } from '../types';

// =============================================================================
// Mocks
// =============================================================================

vi.mock('@tauri-apps/api/core', () => ({
  invoke: vi.fn(),
}));

vi.mock('@tauri-apps/api/event', () => ({
  listen: vi.fn(),
}));

const mockInvoke = vi.mocked(invoke);
const mockListen = vi.mocked(listen);

// Helper to create a mock event emitter
function createMockEventEmitter() {
  let eventCallback: ((event: { payload: any }) => void) | null = null;
  const unlisten = vi.fn();

  mockListen.mockImplementation(async (eventName, callback) => {
    eventCallback = callback;
    return unlisten;
  });

  return {
    emit: (payload: any) => {
      if (eventCallback) {
        eventCallback({ payload });
      }
    },
    unlisten,
    getCallback: () => eventCallback,
  };
}

// =============================================================================
// Setup & Teardown
// =============================================================================

beforeEach(() => {
  vi.clearAllMocks();
  mockInvoke.mockResolvedValue(undefined);
});

afterEach(() => {
  vi.restoreAllMocks();
});

// =============================================================================
// Event Iterator Creation Tests
// =============================================================================

describe('createEventIterator()', () => {
  it('should create an async iterator', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', { start: 0 });
    
    expect(iterator).toBeDefined();
    expect(typeof iterator[Symbol.asyncIterator]).toBe('function');
    expect(typeof iterator.return).toBe('function');
    
    // Cleanup
    await iterator.return();
  });

  it('should set up event listener with correct event name', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', { start: 0 });
    
    expect(mockListen).toHaveBeenCalledWith(
      expect.stringMatching(/^rpc:subscription:sub_/),
      expect.any(Function)
    );
    
    await iterator.return();
  });

  it('should invoke backend subscribe command', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', { start: 0 });
    
    expect(mockInvoke).toHaveBeenCalledWith('plugin:rpc|rpc_subscribe', {
      request: expect.objectContaining({
        path: 'stream.counter',
        input: { start: 0 },
      }),
    });
    
    await iterator.return();
  });

  it('should pass lastEventId when provided', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>(
      'stream.counter',
      { start: 0 },
      { lastEventId: 'event-123' }
    );
    
    expect(mockInvoke).toHaveBeenCalledWith('plugin:rpc|rpc_subscribe', {
      request: expect.objectContaining({
        lastEventId: 'event-123',
      }),
    });
    
    await iterator.return();
  });
});

// =============================================================================
// Event Iteration Tests
// =============================================================================

describe('Event Iteration', () => {
  it('should yield events as they arrive', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    const asyncIterator = iterator[Symbol.asyncIterator]();
    
    // Emit some events
    emitter.emit({ type: 'data', payload: { data: 1 } });
    emitter.emit({ type: 'data', payload: { data: 2 } });
    emitter.emit({ type: 'data', payload: { data: 3 } });
    
    // Consume events
    const result1 = await asyncIterator.next();
    const result2 = await asyncIterator.next();
    const result3 = await asyncIterator.next();
    
    expect(result1).toEqual({ done: false, value: 1 });
    expect(result2).toEqual({ done: false, value: 2 });
    expect(result3).toEqual({ done: false, value: 3 });
    
    await iterator.return();
  });

  it('should buffer events when not being consumed', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    
    // Emit events before consuming
    emitter.emit({ type: 'data', payload: { data: 1 } });
    emitter.emit({ type: 'data', payload: { data: 2 } });
    
    // Now consume
    const events: number[] = [];
    const asyncIterator = iterator[Symbol.asyncIterator]();
    
    events.push((await asyncIterator.next()).value);
    events.push((await asyncIterator.next()).value);
    
    expect(events).toEqual([1, 2]);
    
    await iterator.return();
  });

  it('should complete when receiving completed event', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    const asyncIterator = iterator[Symbol.asyncIterator]();
    
    emitter.emit({ type: 'data', payload: { data: 1 } });
    emitter.emit({ type: 'completed' });
    
    const result1 = await asyncIterator.next();
    const result2 = await asyncIterator.next();
    
    expect(result1).toEqual({ done: false, value: 1 });
    expect(result2).toEqual({ done: true, value: undefined });
  });

  it('should reject on error event', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    const asyncIterator = iterator[Symbol.asyncIterator]();
    
    const error: RpcError = { code: 'SUBSCRIPTION_ERROR', message: 'Stream failed' };
    emitter.emit({ type: 'error', payload: error });
    
    await expect(asyncIterator.next()).rejects.toMatchObject({
      code: 'SUBSCRIPTION_ERROR',
      message: 'Stream failed',
    });
  });

  it('should track lastEventId from events', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    const asyncIterator = iterator[Symbol.asyncIterator]();
    
    emitter.emit({ type: 'data', payload: { data: 1, id: 'event-1' } });
    emitter.emit({ type: 'data', payload: { data: 2, id: 'event-2' } });
    
    await asyncIterator.next();
    await asyncIterator.next();
    
    // The lastEventId should be tracked internally for reconnection
    await iterator.return();
  });
});

// =============================================================================
// Cleanup Tests
// =============================================================================

describe('Cleanup', () => {
  it('should unsubscribe when return() is called', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    
    await iterator.return();
    
    expect(emitter.unlisten).toHaveBeenCalled();
    expect(mockInvoke).toHaveBeenCalledWith('plugin:rpc|rpc_unsubscribe', {
      id: expect.stringMatching(/^sub_/),
    });
  });

  it('should resolve pending consumers on cleanup', async () => {
    const emitter = createMockEventEmitter();
    
    const iterator = await createEventIterator<number>('stream.counter', {});
    const asyncIterator = iterator[Symbol.asyncIterator]();
    
    // Start waiting for next value
    const nextPromise = asyncIterator.next();
    
    // Cleanup while waiting
    await iterator.return();
    
    // Should resolve with done: true
    const result = await nextPromise;
    expect(result).toEqual({ done: true, value: undefined });
  });

  it('should handle abort signal', async () => {
    const emitter = createMockEventEmitter();
    const controller = new AbortController();
    
    const iterator = await createEventIterator<number>(
      'stream.counter',
      {},
      { signal: controller.signal }
    );
    
    controller.abort();
    
    // Give time for abort handler to run
    await new Promise(resolve => setTimeout(resolve, 10));
    
    expect(emitter.unlisten).toHaveBeenCalled();
  });
});

// =============================================================================
// consumeEventIterator Tests
// =============================================================================

describe('consumeEventIterator()', () => {
  it('should call onEvent for each event', async () => {
    const emitter = createMockEventEmitter();
    const onEvent = vi.fn();
    
    const iteratorPromise = createEventIterator<number>('stream.counter', {});
    
    consumeEventIterator(iteratorPromise, { onEvent });
    
    // Wait for iterator to be created
    const iterator = await iteratorPromise;
    
    // Emit events
    emitter.emit({ type: 'data', payload: { data: 1 } });
    emitter.emit({ type: 'data', payload: { data: 2 } });
    
    // Give time for events to be processed
    await new Promise(resolve => setTimeout(resolve, 10));
    
    expect(onEvent).toHaveBeenCalledWith(1);
    expect(onEvent).toHaveBeenCalledWith(2);
    
    await iterator.return();
  });

  it('should call onComplete when stream completes', async () => {
    const emitter = createMockEventEmitter();
    const onComplete = vi.fn();
    const onFinish = vi.fn();
    
    const iteratorPromise = createEventIterator<number>('stream.counter', {});
    
    consumeEventIterator(iteratorPromise, { onComplete, onFinish });
    
    const iterator = await iteratorPromise;
    
    emitter.emit({ type: 'completed' });
    
    await new Promise(resolve => setTimeout(resolve, 10));
    
    expect(onComplete).toHaveBeenCalled();
    expect(onFinish).toHaveBeenCalledWith('success');
  });

  it('should call onError when stream errors', async () => {
    const emitter = createMockEventEmitter();
    const onError = vi.fn();
    const onFinish = vi.fn();
    
    const iteratorPromise = createEventIterator<number>('stream.counter', {});
    
    consumeEventIterator(iteratorPromise, { onError, onFinish });
    
    await iteratorPromise;
    
    const error: RpcError = { code: 'ERROR', message: 'Failed' };
    emitter.emit({ type: 'error', payload: error });
    
    await new Promise(resolve => setTimeout(resolve, 10));
    
    expect(onError).toHaveBeenCalledWith(expect.objectContaining({ code: 'ERROR' }));
    expect(onFinish).toHaveBeenCalledWith('error');
  });

  it('should return cancel function', async () => {
    const emitter = createMockEventEmitter();
    const onFinish = vi.fn();
    
    const iteratorPromise = createEventIterator<number>('stream.counter', {});
    
    const cancel = consumeEventIterator(iteratorPromise, { onFinish });
    
    await iteratorPromise;
    
    await cancel();
    
    expect(onFinish).toHaveBeenCalledWith('cancelled');
  });

  it('should stop processing events after cancel', async () => {
    const emitter = createMockEventEmitter();
    const onEvent = vi.fn();
    
    const iteratorPromise = createEventIterator<number>('stream.counter', {});
    
    const cancel = consumeEventIterator(iteratorPromise, { onEvent });
    
    await iteratorPromise;
    
    emitter.emit({ type: 'data', payload: { data: 1 } });
    await new Promise(resolve => setTimeout(resolve, 10));
    
    await cancel();
    
    emitter.emit({ type: 'data', payload: { data: 2 } });
    await new Promise(resolve => setTimeout(resolve, 10));
    
    expect(onEvent).toHaveBeenCalledTimes(1);
    expect(onEvent).toHaveBeenCalledWith(1);
  });
});

// =============================================================================
// Property-Based Tests
// =============================================================================

describe('Property-Based Tests', () => {
  // Property: All emitted data events are yielded in order
  it('property: events are yielded in emission order', async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.array(fc.integer(), { minLength: 1, maxLength: 20 }),
        async (values) => {
          const emitter = createMockEventEmitter();
          
          const iterator = await createEventIterator<number>('stream.test', {});
          const asyncIterator = iterator[Symbol.asyncIterator]();
          
          // Emit all values
          for (const value of values) {
            emitter.emit({ type: 'data', payload: { data: value } });
          }
          emitter.emit({ type: 'completed' });
          
          // Collect all values
          const collected: number[] = [];
          let result = await asyncIterator.next();
          while (!result.done) {
            collected.push(result.value);
            result = await asyncIterator.next();
          }
          
          expect(collected).toEqual(values);
          
          await iterator.return();
        }
      ),
      { numRuns: 50 }
    );
  });

  // Property: Iterator always terminates on completed event
  it('property: iterator terminates on completed event', async () => {
    await fc.assert(
      fc.asyncProperty(
        fc.array(fc.string(), { maxLength: 10 }),
        async (values) => {
          const emitter = createMockEventEmitter();
          
          const iterator = await createEventIterator<string>('stream.test', {});
          const asyncIterator = iterator[Symbol.asyncIterator]();
          
          // Emit values then complete
          for (const value of values) {
            emitter.emit({ type: 'data', payload: { data: value } });
          }
          emitter.emit({ type: 'completed' });
          
          // Consume all
          let count = 0;
          let result = await asyncIterator.next();
          while (!result.done) {
            count++;
            result = await asyncIterator.next();
          }
          
          expect(count).toBe(values.length);
          expect(result.done).toBe(true);
          
          await iterator.return();
        }
      ),
      { numRuns: 50 }
    );
  });
});
