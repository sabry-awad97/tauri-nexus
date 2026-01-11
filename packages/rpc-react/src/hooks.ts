// =============================================================================
// Tauri RPC Client - React Hooks
// =============================================================================
// Subscription hook for streaming data. For queries and mutations,
// use TanStack Query directly with the RPC client.

import { useState, useEffect, useCallback, useRef } from "react";
import type {
  RpcError,
  EventIterator,
  SubscriptionOptions as BaseSubscriptionOptions,
  BatchCallOptions,
  TypedBatchResult,
} from "@tauri-nexus/rpc-core";
import {
  isRpcError,
  TypedBatchBuilder,
  TypedBatchResponseWrapper,
} from "@tauri-nexus/rpc-core";

// =============================================================================
// Subscription Hook Types
// =============================================================================

export interface SubscriptionState<T> {
  data: T[];
  latestEvent: T | undefined;
  error: RpcError | null;
  isConnected: boolean;
  isConnecting: boolean;
  isError: boolean;
  connectionCount: number;
}

export interface SubscriptionResult<T> extends SubscriptionState<T> {
  unsubscribe: () => void;
  clear: () => void;
  reconnect: () => void;
}

export interface SubscriptionHookOptions<T> extends BaseSubscriptionOptions {
  /** Whether the subscription should connect */
  enabled?: boolean;
  /** Maximum events to keep in buffer */
  maxEvents?: number;
  /** Called for each event */
  onEvent?: (event: T) => void;
  /** Called on error */
  onError?: (error: RpcError) => void;
  /** Called when stream completes */
  onComplete?: () => void;
  /** Called on connect */
  onConnect?: () => void;
  /** Called on disconnect */
  onDisconnect?: () => void;
}

// =============================================================================
// useSubscription Hook
// =============================================================================

/**
 * React hook for RPC subscriptions with automatic connection management.
 *
 * For queries and mutations, use TanStack Query directly:
 * ```typescript
 * import { useQuery, useMutation } from '@tanstack/react-query';
 *
 * // Query
 * const { data } = useQuery({
 *   queryKey: ['user', id],
 *   queryFn: () => rpc.user.get({ id }),
 * });
 *
 * // Mutation
 * const { mutate } = useMutation({
 *   mutationFn: (input) => rpc.user.create(input),
 * });
 * ```
 *
 * @example
 * ```typescript
 * const { data, latestEvent, isConnected, unsubscribe } = useSubscription(
 *   () => rpc.stream.counter({ start: 0 }),
 *   [startValue],
 *   {
 *     onEvent: (event) => console.log('Count:', event.count),
 *     maxEvents: 100,
 *   }
 * );
 * ```
 */
export function useSubscription<T>(
  subscribeFn: () => Promise<EventIterator<T>>,
  deps: unknown[] = [],
  options: SubscriptionHookOptions<T> = {},
): SubscriptionResult<T> {
  const {
    enabled = true,
    maxEvents = 1000,
    onEvent,
    onError,
    onComplete,
    onConnect,
    onDisconnect,
    autoReconnect = false,
    reconnectDelay = 1000,
    maxReconnects = 5,
  } = options;

  const [state, setState] = useState<SubscriptionState<T>>({
    data: [],
    latestEvent: undefined,
    error: null,
    isConnected: false,
    isConnecting: false,
    isError: false,
    connectionCount: 0,
  });

  const mountedRef = useRef(true);
  const iteratorRef = useRef<EventIterator<T> | null>(null);
  const reconnectCountRef = useRef(0);
  const manualDisconnectRef = useRef(false);
  const connectionIdRef = useRef(0);

  const connect = useCallback(async () => {
    if (!enabled || !mountedRef.current || manualDisconnectRef.current) return;

    const currentConnectionId = ++connectionIdRef.current;

    setState((s) => ({
      ...s,
      isConnecting: true,
      error: null,
      isError: false,
    }));

    try {
      const iterator = await subscribeFn();

      if (
        currentConnectionId !== connectionIdRef.current ||
        !mountedRef.current
      ) {
        await iterator.return();
        return;
      }

      iteratorRef.current = iterator;

      if (mountedRef.current) {
        setState((s) => ({
          ...s,
          isConnected: true,
          isConnecting: false,
          connectionCount: s.connectionCount + 1,
        }));
        onConnect?.();
        reconnectCountRef.current = 0;
      }

      for await (const event of iterator) {
        if (
          !mountedRef.current ||
          currentConnectionId !== connectionIdRef.current
        )
          break;

        setState((s) => {
          const newData = [...s.data, event];
          if (newData.length > maxEvents) {
            newData.splice(0, newData.length - maxEvents);
          }
          return {
            ...s,
            data: newData,
            latestEvent: event,
          };
        });
        onEvent?.(event);
      }

      if (
        mountedRef.current &&
        !manualDisconnectRef.current &&
        currentConnectionId === connectionIdRef.current
      ) {
        setState((s) => ({ ...s, isConnected: false, isConnecting: false }));
        onComplete?.();
        onDisconnect?.();
      }
    } catch (err) {
      if (currentConnectionId !== connectionIdRef.current) return;

      const error = isRpcError(err)
        ? err
        : { code: "UNKNOWN", message: String(err) };

      if (mountedRef.current) {
        setState((s) => ({
          ...s,
          error,
          isConnected: false,
          isConnecting: false,
          isError: true,
        }));
        onError?.(error);
        onDisconnect?.();

        if (
          autoReconnect &&
          !manualDisconnectRef.current &&
          reconnectCountRef.current < maxReconnects
        ) {
          reconnectCountRef.current++;
          const delay =
            reconnectDelay * Math.pow(2, reconnectCountRef.current - 1);
          setTimeout(connect, delay);
        }
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [
    enabled,
    maxEvents,
    autoReconnect,
    reconnectDelay,
    maxReconnects,
    ...deps,
  ]);

  useEffect(() => {
    manualDisconnectRef.current = false;
    connect();
    return () => {
      iteratorRef.current?.return();
    };
  }, [connect]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const unsubscribe = useCallback(() => {
    manualDisconnectRef.current = true;
    iteratorRef.current?.return();
    setState((s) => ({ ...s, isConnected: false, isConnecting: false }));
    onDisconnect?.();
  }, [onDisconnect]);

  const clear = useCallback(() => {
    setState((s) => ({ ...s, data: [], latestEvent: undefined }));
  }, []);

  const reconnect = useCallback(() => {
    manualDisconnectRef.current = false;
    reconnectCountRef.current = 0;
    iteratorRef.current?.return();
    connect();
  }, [connect]);

  return { ...state, unsubscribe, clear, reconnect };
}

// =============================================================================
// Utility Hooks
// =============================================================================

/**
 * Hook to track if component is mounted (useful for async operations)
 */
export function useIsMounted(): () => boolean {
  const mountedRef = useRef(false);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return useCallback(() => mountedRef.current, []);
}

// =============================================================================
// Batch Hook Types
// =============================================================================

export interface BatchState<TOutputMap extends Record<string, unknown>> {
  /** The batch response wrapper (null before execution) */
  response: TypedBatchResponseWrapper<TOutputMap> | null;
  /** Whether the batch is currently executing */
  isLoading: boolean;
  /** Error if the entire batch failed */
  error: RpcError | null;
  /** Whether there was an error */
  isError: boolean;
  /** Whether the batch has been executed successfully */
  isSuccess: boolean;
  /** Execution duration in milliseconds */
  duration: number | null;
}

export interface BatchResult<
  TOutputMap extends Record<string, unknown>,
> extends BatchState<TOutputMap> {
  /** Execute the batch */
  execute: (
    options?: BatchCallOptions,
  ) => Promise<TypedBatchResponseWrapper<TOutputMap>>;
  /** Reset the state */
  reset: () => void;
  /** Get a typed result by ID (shorthand for response?.getResult) */
  getResult: <TId extends keyof TOutputMap & string>(
    id: TId,
  ) => TypedBatchResult<TOutputMap[TId]> | undefined;
}

export interface UseBatchOptions {
  /** Execute immediately on mount */
  executeOnMount?: boolean;
  /** Called when batch succeeds */
  onSuccess?: (
    response: TypedBatchResponseWrapper<Record<string, unknown>>,
  ) => void;
  /** Called when batch fails */
  onError?: (error: RpcError) => void;
}

// =============================================================================
// useBatch Hook
// =============================================================================

/**
 * React hook for executing type-safe batch operations.
 *
 * This hook provides React state management for batch operations,
 * including loading states, error handling, and result access.
 *
 * @example
 * ```typescript
 * function MyComponent() {
 *   const batch = useBatch(
 *     () => rpc.batch()
 *       .add("health", "health", undefined)
 *       .add("user", "user.get", { id: 1 })
 *       .add("greeting", "greet", { name: "World" }),
 *     { executeOnMount: true }
 *   );
 *
 *   if (batch.isLoading) return <div>Loading...</div>;
 *   if (batch.isError) return <div>Error: {batch.error?.message}</div>;
 *
 *   const healthResult = batch.getResult("health");
 *   const userResult = batch.getResult("user");
 *
 *   return (
 *     <div>
 *       <p>Health: {healthResult?.data?.status}</p>
 *       <p>User: {userResult?.data?.name}</p>
 *       <p>Duration: {batch.duration}ms</p>
 *       <button onClick={() => batch.execute()}>Refresh</button>
 *     </div>
 *   );
 * }
 * ```
 *
 * @example Manual execution
 * ```typescript
 * function MyComponent() {
 *   const batch = useBatch(
 *     () => rpc.batch()
 *       .add("users", "user.list", undefined)
 *   );
 *
 *   return (
 *     <button onClick={() => batch.execute()} disabled={batch.isLoading}>
 *       {batch.isLoading ? "Loading..." : "Fetch Users"}
 *     </button>
 *   );
 * }
 * ```
 */
export function useBatch<TContract, TOutputMap extends Record<string, unknown>>(
  builderFn: () => TypedBatchBuilder<TContract, TOutputMap>,
  options: UseBatchOptions = {},
): BatchResult<TOutputMap> {
  const { executeOnMount = false, onSuccess, onError } = options;

  const [state, setState] = useState<BatchState<TOutputMap>>({
    response: null,
    isLoading: false,
    error: null,
    isError: false,
    isSuccess: false,
    duration: null,
  });

  const mountedRef = useRef(true);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  const execute = useCallback(
    async (callOptions?: BatchCallOptions) => {
      setState((s) => ({
        ...s,
        isLoading: true,
        error: null,
        isError: false,
      }));

      const startTime = performance.now();

      try {
        const builder = builderFn();
        const response = await builder.execute(callOptions);
        const duration = performance.now() - startTime;

        if (mountedRef.current) {
          setState({
            response,
            isLoading: false,
            error: null,
            isError: false,
            isSuccess: true,
            duration,
          });
          onSuccess?.(
            response as TypedBatchResponseWrapper<Record<string, unknown>>,
          );
        }

        return response;
      } catch (err) {
        const error = isRpcError(err)
          ? err
          : { code: "UNKNOWN", message: String(err) };
        const duration = performance.now() - startTime;

        if (mountedRef.current) {
          setState({
            response: null,
            isLoading: false,
            error,
            isError: true,
            isSuccess: false,
            duration,
          });
          onError?.(error);
        }

        throw error;
      }
    },
    [builderFn, onSuccess, onError],
  );

  const reset = useCallback(() => {
    setState({
      response: null,
      isLoading: false,
      error: null,
      isError: false,
      isSuccess: false,
      duration: null,
    });
  }, []);

  const getResult = useCallback(
    <TId extends keyof TOutputMap & string>(id: TId) => {
      return state.response?.getResult(id);
    },
    [state.response],
  );

  // Execute on mount if requested
  useEffect(() => {
    if (executeOnMount) {
      execute();
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  return {
    ...state,
    execute,
    reset,
    getResult,
  };
}
