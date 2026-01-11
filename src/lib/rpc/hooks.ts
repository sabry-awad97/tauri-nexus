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
} from "./types";
import { isRpcError } from "./client";

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
