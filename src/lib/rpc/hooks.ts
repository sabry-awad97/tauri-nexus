// =============================================================================
// React Hooks for Type-Safe RPC
// =============================================================================

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  type ReactNode,
} from 'react';
import type { 
  RpcError, 
  ContractRouter, 
  RouterClient,
  EventIterator,
  InferInput,
  InferOutput,
  ProcedureDef,
  QueryDef,
  MutationDef,
  SubscriptionDef,
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
}

export interface QueryResult<T> extends QueryState<T> {
  refetch: () => Promise<void>;
}

export interface QueryOptions {
  enabled?: boolean;
  refetchInterval?: number;
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
  onSettled?: () => void;
}

// =============================================================================
// Subscription Hook Types
// =============================================================================

export interface SubscriptionState<T> {
  data: T[];
  latestEvent: T | undefined;
  error: RpcError | null;
  isConnected: boolean;
  isError: boolean;
}

export interface SubscriptionResult<T> extends SubscriptionState<T> {
  unsubscribe: () => void;
  clear: () => void;
}

export interface SubscriptionOptions<T> {
  enabled?: boolean;
  lastEventId?: string;
  autoReconnect?: boolean;
  reconnectDelay?: number;
  maxReconnects?: number;
  onEvent?: (event: T) => void;
  onError?: (error: RpcError) => void;
  onComplete?: () => void;
}

// =============================================================================
// Generic Hooks
// =============================================================================

/** Generic query hook */
export function useQuery<T>(
  queryFn: () => Promise<T>,
  deps: unknown[],
  options: QueryOptions = {}
): QueryResult<T> {
  const { enabled = true, refetchInterval } = options;
  const [state, setState] = useState<QueryState<T>>({
    data: undefined,
    error: null,
    isLoading: enabled,
    isError: false,
    isSuccess: false,
  });
  const mountedRef = useRef(true);

  const fetchData = useCallback(async () => {
    if (!enabled) return;
    
    setState(s => ({ ...s, isLoading: true, error: null }));

    try {
      const result = await queryFn();
      if (mountedRef.current) {
        setState({
          data: result,
          error: null,
          isLoading: false,
          isError: false,
          isSuccess: true,
        });
      }
    } catch (err) {
      if (mountedRef.current) {
        const error = isRpcError(err) ? err : { code: 'UNKNOWN', message: String(err) };
        setState(s => ({
          ...s,
          error,
          isLoading: false,
          isError: true,
          isSuccess: false,
        }));
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, ...deps]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

  useEffect(() => {
    if (!refetchInterval || !enabled) return;
    const interval = setInterval(fetchData, refetchInterval);
    return () => clearInterval(interval);
  }, [refetchInterval, enabled, fetchData]);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  return { ...state, refetch: fetchData };
}

/** Generic mutation hook */
export function useMutation<TInput, TOutput>(
  mutationFn: (input: TInput) => Promise<TOutput>,
  options: MutationOptions<TInput, TOutput> = {}
): MutationResult<TInput, TOutput> {
  const { onSuccess, onError, onSettled } = options;
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
      setState(s => ({ ...s, isLoading: true, isIdle: false, error: null }));

      try {
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
        }
        onSettled?.();
        return result;
      } catch (err) {
        const error = isRpcError(err) ? err : { code: 'UNKNOWN', message: String(err) };
        if (mountedRef.current) {
          setState(s => ({
            ...s,
            error,
            isLoading: false,
            isError: true,
            isSuccess: false,
          }));
          onError?.(error, input);
        }
        onSettled?.();
        throw error;
      }
    },
    [mutationFn, onSuccess, onError, onSettled]
  );

  const mutate = useCallback(
    (input: TInput) => { mutateAsync(input).catch(() => {}); },
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
    return () => { mountedRef.current = false; };
  }, []);

  return { ...state, mutate, mutateAsync, reset };
}

/** Generic subscription hook */
export function useSubscription<T>(
  subscribeFn: () => Promise<EventIterator<T>>,
  deps: unknown[],
  options: SubscriptionOptions<T> = {}
): SubscriptionResult<T> {
  const { 
    enabled = true, 
    onEvent, 
    onError, 
    onComplete,
    autoReconnect = false,
    reconnectDelay = 1000,
    maxReconnects = 5,
  } = options;
  
  const [state, setState] = useState<SubscriptionState<T>>({
    data: [],
    latestEvent: undefined,
    error: null,
    isConnected: false,
    isError: false,
  });
  
  const mountedRef = useRef(true);
  const iteratorRef = useRef<EventIterator<T> | null>(null);
  const reconnectCountRef = useRef(0);

  const connect = useCallback(async () => {
    if (!enabled || !mountedRef.current) return;

    try {
      const iterator = await subscribeFn();
      iteratorRef.current = iterator;
      
      if (mountedRef.current) {
        setState(s => ({ ...s, isConnected: true, error: null, isError: false }));
      }

      for await (const event of iterator) {
        if (!mountedRef.current) break;
        
        setState(s => ({
          ...s,
          data: [...s.data, event],
          latestEvent: event,
        }));
        onEvent?.(event);
      }

      // Stream completed normally
      if (mountedRef.current) {
        setState(s => ({ ...s, isConnected: false }));
        onComplete?.();
      }
    } catch (err) {
      const error = isRpcError(err) ? err : { code: 'UNKNOWN', message: String(err) };
      
      if (mountedRef.current) {
        setState(s => ({ ...s, error, isConnected: false, isError: true }));
        onError?.(error);

        // Auto-reconnect logic
        if (autoReconnect && reconnectCountRef.current < maxReconnects) {
          reconnectCountRef.current++;
          setTimeout(connect, reconnectDelay);
        }
      }
    }
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, ...deps]);

  useEffect(() => {
    connect();
    
    return () => {
      iteratorRef.current?.return();
    };
  }, [connect]);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  const unsubscribe = useCallback(() => {
    iteratorRef.current?.return();
    setState(s => ({ ...s, isConnected: false }));
  }, []);

  const clear = useCallback(() => {
    setState(s => ({ ...s, data: [], latestEvent: undefined }));
  }, []);

  return { ...state, unsubscribe, clear };
}

// =============================================================================
// Hook Factory for Type-Safe Hooks
// =============================================================================

/** Create typed hooks from a contract */
export function createHooks<T extends ContractRouter>(
  client: RouterClient<T>
) {
  return {
    /** Create a query hook for a specific procedure */
    useQuery: <P extends QueryDef<any, any>>(
      procedure: (client: RouterClient<T>) => (input: InferInput<P>) => Promise<InferOutput<P>>,
      input: InferInput<P>,
      options?: QueryOptions
    ): QueryResult<InferOutput<P>> => {
      const fn = procedure(client);
      return useQuery(() => fn(input), [JSON.stringify(input)], options);
    },

    /** Create a mutation hook for a specific procedure */
    useMutation: <P extends MutationDef<any, any>>(
      procedure: (client: RouterClient<T>) => (input: InferInput<P>) => Promise<InferOutput<P>>,
      options?: MutationOptions<InferInput<P>, InferOutput<P>>
    ): MutationResult<InferInput<P>, InferOutput<P>> => {
      const fn = procedure(client);
      return useMutation(fn, options);
    },

    /** Create a subscription hook for a specific procedure */
    useSubscription: <P extends SubscriptionDef<any, any>>(
      procedure: (client: RouterClient<T>) => (input: InferInput<P>) => Promise<EventIterator<InferOutput<P>>>,
      input: InferInput<P>,
      options?: SubscriptionOptions<InferOutput<P>>
    ): SubscriptionResult<InferOutput<P>> => {
      const fn = procedure(client);
      return useSubscription(() => fn(input), [JSON.stringify(input)], options);
    },
  };
}
