// =============================================================================
// Tauri RPC Client - Event Iterator Implementation
// =============================================================================
// Robust async iterator for subscription streams with auto-reconnect support.

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type {
  Event,
  EventIterator,
  RpcError,
  SubscriptionOptions,
  SubscribeRequest,
} from './types';

// =============================================================================
// Types
// =============================================================================

/** Subscription event from backend (matches Rust SubscriptionEvent) */
interface SubscriptionEvent<T> {
  type: 'data' | 'error' | 'completed';
  payload?: Event<T> | RpcError;
}

/** Internal subscription state */
interface SubscriptionState<T> {
  id: string;
  path: string;
  input: unknown;
  options: SubscriptionOptions;
  queue: Array<{
    resolve: (value: IteratorResult<T>) => void;
    reject: (error: RpcError) => void;
  }>;
  buffer: T[];
  error: RpcError | null;
  completed: boolean;
  unlisten: UnlistenFn | null;
  reconnectAttempts: number;
  lastEventId?: string;
}

// =============================================================================
// Event Iterator Factory
// =============================================================================

/**
 * Create an async event iterator for a subscription.
 *
 * @example
 * ```typescript
 * const iterator = await createEventIterator<CounterEvent>('stream.counter', { start: 0 });
 *
 * for await (const event of iterator) {
 *   console.log('Count:', event.count);
 * }
 * ```
 */
export async function createEventIterator<T>(
  path: string,
  input: unknown = null,
  options: SubscriptionOptions = {}
): Promise<EventIterator<T>> {
  const subscriptionId = generateSubscriptionId();

  const state: SubscriptionState<T> = {
    id: subscriptionId,
    path,
    input,
    options,
    queue: [],
    buffer: [],
    error: null,
    completed: false,
    unlisten: null,
    reconnectAttempts: 0,
    lastEventId: options.lastEventId,
  };

  // Initial connection
  await connect(state);

  // Handle abort signal
  if (options.signal) {
    options.signal.addEventListener('abort', () => {
      cleanup(state);
    });
  }

  // Create the async iterator
  const iterator: EventIterator<T> = {
    async return(): Promise<void> {
      await cleanup(state);
    },

    [Symbol.asyncIterator](): AsyncIterator<T> {
      return {
        async next(): Promise<IteratorResult<T>> {
          return getNextValue(state);
        },
        async return(): Promise<IteratorResult<T>> {
          await cleanup(state);
          return { done: true, value: undefined };
        },
      };
    },
  };

  return iterator;
}

// =============================================================================
// Connection Management
// =============================================================================

/** Generate a unique subscription ID */
function generateSubscriptionId(): string {
  // Use crypto.randomUUID if available, otherwise fallback
  if (typeof crypto !== 'undefined' && crypto.randomUUID) {
    return crypto.randomUUID();
  }
  // Fallback for older environments
  return 'xxxxxxxx-xxxx-4xxx-yxxx-xxxxxxxxxxxx'.replace(/[xy]/g, (c) => {
    const r = (Math.random() * 16) | 0;
    const v = c === 'x' ? r : (r & 0x3) | 0x8;
    return v.toString(16);
  });
}

/** Connect to the subscription */
async function connect<T>(state: SubscriptionState<T>): Promise<void> {
  // Set up event listener first
  const eventName = `rpc:subscription:sub_${state.id}`;
  state.unlisten = await listen<SubscriptionEvent<T>>(eventName, (event) => {
    handleEvent(state, event.payload);
  });

  // Start subscription on backend
  const request: SubscribeRequest = {
    id: state.id,
    path: state.path,
    input: state.input,
    lastEventId: state.lastEventId,
  };

  try {
    await invoke('plugin:rpc|rpc_subscribe', { request });
  } catch (error) {
    // Clean up listener if subscription fails
    if (state.unlisten) {
      state.unlisten();
      state.unlisten = null;
    }
    throw error;
  }
}

/** Attempt to reconnect */
async function reconnect<T>(state: SubscriptionState<T>): Promise<boolean> {
  const { autoReconnect = false, maxReconnects = 5, reconnectDelay = 1000 } =
    state.options;

  if (!autoReconnect || state.reconnectAttempts >= maxReconnects) {
    return false;
  }

  state.reconnectAttempts++;

  // Calculate backoff delay with jitter
  const delay = reconnectDelay * Math.pow(2, state.reconnectAttempts - 1);
  const jitteredDelay = delay * (0.5 + Math.random() * 0.5);

  await sleep(jitteredDelay);

  try {
    // Generate new subscription ID for reconnect
    state.id = generateSubscriptionId();
    state.completed = false;
    state.error = null;

    await connect(state);
    state.reconnectAttempts = 0; // Reset on successful reconnect
    return true;
  } catch {
    return reconnect(state); // Try again
  }
}

// =============================================================================
// Event Handling
// =============================================================================

/** Handle incoming subscription event */
function handleEvent<T>(
  state: SubscriptionState<T>,
  event: SubscriptionEvent<T>
): void {
  switch (event.type) {
    case 'data': {
      const eventData = event.payload as Event<T>;
      const data = eventData.data;

      // Track last event ID for resumption
      if (eventData.id) {
        state.lastEventId = eventData.id;
      }

      // If someone is waiting, resolve immediately
      if (state.queue.length > 0) {
        const { resolve } = state.queue.shift()!;
        resolve({ done: false, value: data });
      } else {
        // Otherwise buffer the value
        state.buffer.push(data);
      }
      break;
    }

    case 'error': {
      state.error = event.payload as RpcError;
      state.completed = true;

      // Reject all waiting consumers
      while (state.queue.length > 0) {
        const { reject } = state.queue.shift()!;
        reject(state.error);
      }

      // Attempt reconnect if enabled
      if (state.options.autoReconnect) {
        reconnect(state).then((reconnected) => {
          if (!reconnected) {
            // Final failure - already rejected waiting consumers
          }
        });
      }
      break;
    }

    case 'completed': {
      state.completed = true;

      // Resolve all waiting consumers with done
      while (state.queue.length > 0) {
        const { resolve } = state.queue.shift()!;
        resolve({ done: true, value: undefined });
      }
      break;
    }
  }
}

/** Get the next value from the iterator */
function getNextValue<T>(
  state: SubscriptionState<T>
): Promise<IteratorResult<T>> {
  // If there's an error, throw it
  if (state.error) {
    return Promise.reject(state.error);
  }

  // If completed and buffer is empty, we're done
  if (state.completed && state.buffer.length === 0) {
    return Promise.resolve({ done: true, value: undefined });
  }

  // If there's buffered data, return it
  if (state.buffer.length > 0) {
    const value = state.buffer.shift()!;
    return Promise.resolve({ done: false, value });
  }

  // Otherwise, wait for the next event
  return new Promise((resolve, reject) => {
    state.queue.push({ resolve, reject });
  });
}

/** Clean up subscription resources */
async function cleanup<T>(state: SubscriptionState<T>): Promise<void> {
  // Remove event listener
  if (state.unlisten) {
    state.unlisten();
    state.unlisten = null;
  }

  state.completed = true;

  // Notify backend to cancel
  try {
    await invoke('plugin:rpc|rpc_unsubscribe', { id: `sub_${state.id}` });
  } catch {
    // Ignore errors during cleanup
  }

  // Resolve any waiting consumers
  while (state.queue.length > 0) {
    const { resolve } = state.queue.shift()!;
    resolve({ done: true, value: undefined });
  }
}

// =============================================================================
// Utility: consumeEventIterator
// =============================================================================

export interface ConsumeOptions<T> {
  /** Called for each event */
  onEvent?: (event: T) => void;
  /** Called on error */
  onError?: (error: RpcError) => void;
  /** Called when stream completes successfully */
  onComplete?: () => void;
  /** Called when stream finishes (success, error, or cancelled) */
  onFinish?: (state: 'success' | 'error' | 'cancelled') => void;
}

/**
 * Consume an event iterator with lifecycle callbacks.
 * Returns a cancel function.
 *
 * @example
 * ```typescript
 * const cancel = consumeEventIterator(
 *   rpc.stream.counter({ start: 0 }),
 *   {
 *     onEvent: (event) => console.log('Count:', event.count),
 *     onError: (error) => console.error('Error:', error),
 *     onComplete: () => console.log('Stream completed'),
 *   }
 * );
 *
 * // Later: cancel the subscription
 * await cancel();
 * ```
 */
export function consumeEventIterator<T>(
  iteratorPromise: Promise<EventIterator<T>>,
  options: ConsumeOptions<T>
): () => Promise<void> {
  let cancelled = false;
  let iterator: EventIterator<T> | null = null;

  // Start consuming
  (async () => {
    try {
      iterator = await iteratorPromise;

      for await (const event of iterator) {
        if (cancelled) break;
        options.onEvent?.(event);
      }

      if (!cancelled) {
        options.onComplete?.();
        options.onFinish?.('success');
      }
    } catch (error) {
      if (!cancelled) {
        options.onError?.(error as RpcError);
        options.onFinish?.('error');
      }
    }
  })();

  // Return cancel function
  return async () => {
    cancelled = true;
    if (iterator) {
      await iterator.return();
    }
    options.onFinish?.('cancelled');
  };
}

// =============================================================================
// Helpers
// =============================================================================

/** Sleep utility */
function sleep(ms: number): Promise<void> {
  return new Promise((resolve) => setTimeout(resolve, ms));
}
