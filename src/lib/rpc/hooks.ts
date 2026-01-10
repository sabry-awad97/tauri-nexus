// =============================================================================
// Tauri RPC Client - React Hooks
// =============================================================================
// Type-safe React hooks for queries, mutations, and subscriptions.

import { useState, useEffect, useCallback, useRef, useMemo } from 'react';
import type {
  RpcError,
  RouterClient,
  EventIterator,
  CallOptions,
  SubscriptionOptions as BaseSubscriptionOptions,
} from './types';
import { isRpcError } from './client';

// =============================================================================
// Query Hook Types
// =============================================================================

export interface QueryState<T> {
  data: T | undefined;
  error: RpcError | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  isFetching: boolean;
}

export interface QueryResult<T> extends QueryState<T> {
  refetch: () => Promise<void>;
}

export interface QueryOptions extends CallOptions {
  /** Whether the query should execute */
  enabled?: boolean;
  /** Refetch interval in milliseconds */
  refetchInterval?: number;
  /** Keep previous data while refetching */
  keepPreviousData?: boolean;
  /** Initial data */
  initialData?: unknown;
}

// =============================================================================
// Mutation Hook Types
// =============================================================================

export interface MutationState<T> {
  data: T | undefined;
  error: RpcError | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  isIdle: boolean;
}

export interface MutationResult<TInput, TOutput> extends MutationState<TOutput> {
  mutate: (input: TInput) => void;
  mutateAsync: (input: TInput) => Promise<TOutput>;
  reset: () => void;
}

export interface MutationOptions<TInput, TOutput> {
  onSuccess?: (data: TOutput, input: TInput) => void;
  onError?: (error: RpcError, input: TInput) => void;
  onSettled?: (data: TOutput | undefined, error: RpcError | null, input: TInput) => void;
  onMutate?: (input: TInput) => void | Promise<void>;
}

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
// useQuery Hook
// =============================================================================

/**
 * React hook for RPC queries with automatic fetching and caching.
 *
 * @example
 * ```typescript
 * const { data, isLoading, error, refetch } = useQuery(
 *   () => rpc.user.get({ id: 1 }),
 *   [userId],
 *   { enabled: !!userId }
 * );
 * ```
 */
export function useQuery<T>(
  queryFn: () => Promise<T>,
  deps: unknown[] = [],
  options: QueryOptions = {}
): QueryResult<T> {
  const { enabled = true, refetchInterval, keepPreviousData = false, initialData } = options;

  const [state, setState] = useState<QueryState<T>>({
    data: initialData as T | undefined,
    error: null,
    isLoading: enabled && initialData === undefined,
    isError: false,
    isSuccess: initialData !== undefined,
    isFetching: false,
  });

  const mountedRef = useRef(true);
  const previousDataRef = useRef<T | undefined>(undefined);

  const fetchData = useCallback(async () => {
    if (!enabled || !mountedRef.current) return;

    setState((s) => ({
      ...s,
      isFetching: true,
      isLoading: s.data === undefined && !keepPreviousData,
      error: null,
    }));

    try {
      const result = await queryFn();

      if (mountedRef.current) {
        previousDataRef.current = result;
        setState({
          data: result,
          error: null,
          isLoading: false,
          isError: false,
          isSuccess: true,
          isFetching: false,
        });
      }
    } catch (err) {
      if (mountedRef.current) {
        const error = isRpcError(err) ? err : { code: 'UNKNOWN', message: String(err) };
        setState((s) => ({
          ...s,
          data: keepPreviousData ? previousDataRef.current : undefined,
          error,
          isLoading: false,
          isError: true,
          isSuccess: false,
          isFetching: false,
        }));
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, keepPreviousData, ...deps]);

  // Initial fetch
  useEffect(() => {
    fetchData();
  }, [fetchData]);

  // Refetch interval
  useEffect(() => {
    if (!refetchInterval || !enabled) return;
    const interval = setInterval(fetchData, refetchInterval);
    return () => clearInterval(interval);
  }, [refetchInterval, enabled, fetchData]);

  // Cleanup
  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return { ...state, refetch: fetchData };
}

// =============================================================================
// useMutation Hook
// =============================================================================

/**
 * React hook for RPC mutations with loading and error states.
 *
 * @example
 * ```typescript
 * const { mutate, mutateAsync, isLoading } = useMutation(
 *   (input) => rpc.user.create(input),
 *   {
 *     onSuccess: (user) => console.log('Created:', user),
 *     onError: (error) => console.error('Failed:', error),
 *   }
 * );
 *
 * // Fire and forget
 * mutate({ name: 'John', email: 'john@example.com' });
 *
 * // Or await the result
 * const user = await mutateAsync({ name: 'John', email: 'john@example.com' });
 * ```
 */
export function useMutation<TInput, TOutput>(
  mutationFn: (input: TInput) => Promise<TOutput>,
  options: MutationOptions<TInput, TOutput> = {}
): MutationResult<TInput, TOutput> {
  const { onSuccess, onError, onSettled, onMutate } = options;

  const [state, setState] = useState<MutationState<TOutput>>({
    data: undefined,
    error: null,
    isLoading: false,
    isError: false,
    isSuccess: false,
    isIdle: true,
  });

  const mountedRef = useRef(true);

  const mutateAsync = useCallback(
    async (input: TInput): Promise<TOutput> => {
      setState((s) => ({ ...s, isLoading: true, isIdle: false, error: null }));

      try {
        await onMutate?.(input);
        const result = await mutationFn(input);

        if (mountedRef.current) {
          setState({
            data: result,
            error: null,
            isLoading: false,
            isError: false,
            isSuccess: true,
            isIdle: false,
          });
          onSuccess?.(result, input);
          onSettled?.(result, null, input);
        }

        return result;
      } catch (err) {
        const error = isRpcError(err) ? err : { code: 'UNKNOWN', message: String(err) };

        if (mountedRef.current) {
          setState((s) => ({
            ...s,
            error,
            isLoading: false,
            isError: true,
            isSuccess: false,
          }));
          onError?.(error, input);
          onSettled?.(undefined, error, input);
        }

        throw error;
      }
    },
    [mutationFn, onSuccess, onError, onSettled, onMutate]
  );

  const mutate = useCallback(
    (input: TInput) => {
      mutateAsync(input).catch(() => {});
    },
    [mutateAsync]
  );

  const reset = useCallback(() => {
    setState({
      data: undefined,
      error: null,
      isLoading: false,
      isError: false,
      isSuccess: false,
      isIdle: true,
    });
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return { ...state, mutate, mutateAsync, reset };
}

// =============================================================================
// useSubscription Hook
// =============================================================================

/**
 * React hook for RPC subscriptions with automatic connection management.
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
  options: SubscriptionHookOptions<T> = {}
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

  const connect = useCallback(async () => {
    if (!enabled || !mountedRef.current || manualDisconnectRef.current) return;

    setState((s) => ({ ...s, isConnecting: true, error: null, isError: false }));

    try {
      const iterator = await subscribeFn();
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
        if (!mountedRef.current) break;

        setState((s) => {
          const newData = [...s.data, event];
          // Trim to maxEvents
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

      // Stream completed normally
      if (mountedRef.current && !manualDisconnectRef.current) {
        setState((s) => ({ ...s, isConnected: false, isConnecting: false }));
        onComplete?.();
        onDisconnect?.();
      }
    } catch (err) {
      const error = isRpcError(err) ? err : { code: 'UNKNOWN', message: String(err) };

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

        // Auto-reconnect logic
        if (autoReconnect && !manualDisconnectRef.current && reconnectCountRef.current < maxReconnects) {
          reconnectCountRef.current++;
          const delay = reconnectDelay * Math.pow(2, reconnectCountRef.current - 1);
          setTimeout(connect, delay);
        }
      }
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, maxEvents, autoReconnect, reconnectDelay, maxReconnects, ...deps]);

  // Connect on mount / deps change
  useEffect(() => {
    manualDisconnectRef.current = false;
    connect();

    return () => {
      iteratorRef.current?.return();
    };
  }, [connect]);

  // Cleanup
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
// Hook Factory - Create Typed Hooks from Contract
// =============================================================================

/**
 * Create a set of typed hooks bound to a specific RPC client.
 *
 * @example
 * ```typescript
 * const rpc = createClient<AppContract>({ subscriptionPaths: ['stream.counter'] });
 * const { useRpcQuery, useRpcMutation, useRpcSubscription } = createHooks(rpc);
 *
 * // In components:
 * const { data } = useRpcQuery((c) => c.user.get({ id: 1 }), [userId]);
 * const { mutate } = useRpcMutation((c) => c.user.create);
 * const { latestEvent } = useRpcSubscription((c) => c.stream.counter({ start: 0 }), []);
 * ```
 */
export function createHooks<T>(client: RouterClient<T>) {
  return {
    /**
     * Query hook with automatic type inference
     */
    useRpcQuery: <TOutput>(
      queryFn: (client: RouterClient<T>) => Promise<TOutput>,
      deps: unknown[] = [],
      options?: QueryOptions
    ): QueryResult<TOutput> => {
      const fn = useMemo(() => () => queryFn(client), [queryFn]);
      return useQuery(fn, deps, options);
    },

    /**
     * Mutation hook with automatic type inference
     */
    useRpcMutation: <TInput, TOutput>(
      getMutationFn: (client: RouterClient<T>) => (input: TInput) => Promise<TOutput>,
      options?: MutationOptions<TInput, TOutput>
    ): MutationResult<TInput, TOutput> => {
      const fn = useMemo(() => getMutationFn(client), [getMutationFn]);
      return useMutation(fn, options);
    },

    /**
     * Subscription hook with automatic type inference
     */
    useRpcSubscription: <TOutput>(
      subscribeFn: (client: RouterClient<T>) => Promise<EventIterator<TOutput>>,
      deps: unknown[] = [],
      options?: SubscriptionHookOptions<TOutput>
    ): SubscriptionResult<TOutput> => {
      const fn = useMemo(() => () => subscribeFn(client), [subscribeFn]);
      return useSubscription(fn, deps, options);
    },

    /** The underlying client */
    client,
  };
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

/**
 * Hook for debounced values (useful for search inputs)
 */
export function useDebounce<T>(value: T, delay: number): T {
  const [debouncedValue, setDebouncedValue] = useState(value);

  useEffect(() => {
    const timer = setTimeout(() => setDebouncedValue(value), delay);
    return () => clearTimeout(timer);
  }, [value, delay]);

  return debouncedValue;
}
