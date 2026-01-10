// =============================================================================
// Event Iterator Implementation
// =============================================================================

import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';
import type { Event, EventIterator, RpcError, SubscriptionOptions } from './types';

// =============================================================================
// Types
// =============================================================================

/** Subscription event from backend */
interface SubscriptionEvent<T> {
  type: 'data' | 'error' | 'completed';
  payload?: Event<T> | RpcError;
}

/** Internal subscription state */
interface SubscriptionState<T> {
  id: string;
  queue: Array<{ resolve: (value: IteratorResult<T>) => void; reject: (error: unknown) => void }>;
  buffer: T[];
  error: RpcError | null;
  completed: boolean;
  unlisten: UnlistenFn | null;
}

// =============================================================================
// Event Iterator Factory
// =============================================================================

/** Create an event iterator for a subscription */
export async function createEventIterator<T>(
  path: string,
  input: unknown = {},
  options?: SubscriptionOptions
): Promise<EventIterator<T>> {
  const subscriptionId = crypto.randomUUID();
  
  const state: SubscriptionState<T> = {
    id: subscriptionId,
    queue: [],
    buffer: [],
    error: null,
    completed: false,
    unlisten: null,
  };

  // Set up event listener
  const eventName = `rpc:subscription:${subscriptionId}`;
  state.unlisten = await listen<SubscriptionEvent<T>>(eventName, (event) => {
    handleEvent(state, event.payload);
  });

  // Start subscription on backend
  await invoke('plugin:rpc|rpc_subscribe', {
    request: {
      id: subscriptionId,
      path,
      input,
      lastEventId: options?.lastEventId,
    }
  });

  // Handle abort signal
  if (options?.signal) {
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
// Internal Helpers
// =============================================================================

/** Handle incoming subscription event */
function handleEvent<T>(state: SubscriptionState<T>, event: SubscriptionEvent<T>): void {
  switch (event.type) {
    case 'data': {
      const data = (event.payload as Event<T>).data;
      
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
function getNextValue<T>(state: SubscriptionState<T>): Promise<IteratorResult<T>> {
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
  if (state.unlisten) {
    state.unlisten();
    state.unlisten = null;
  }
  
  state.completed = true;
  
  // Notify backend to cancel
  try {
    await invoke('plugin:rpc|rpc_unsubscribe', { id: state.id });
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
  onEvent?: (event: T) => void;
  onError?: (error: RpcError) => void;
  onSuccess?: () => void;
  onFinish?: (state: 'success' | 'error' | 'cancelled') => void;
}

/** 
 * Consume an event iterator with lifecycle callbacks
 * Returns a cancel function
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
        options.onSuccess?.();
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
