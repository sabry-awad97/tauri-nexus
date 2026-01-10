// React hooks for generated RPC commands

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  type ReactNode,
} from 'react';
import { rpc } from './router';

type RpcFunctions = typeof rpc;
type RpcKey = keyof RpcFunctions;

// Query result type
export interface UseQueryResult<T> {
  data: T | undefined;
  error: Error | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  refetch: () => Promise<void>;
}

// Mutation result type
export interface UseMutationResult<TInput, TOutput> {
  data: TOutput | undefined;
  error: Error | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  isIdle: boolean;
  mutate: (input: TInput) => void;
  mutateAsync: (input: TInput) => Promise<TOutput>;
  reset: () => void;
}

// Context
const RpcContext = createContext<typeof rpc>(rpc);

export function RpcProvider({ children }: { children: ReactNode }) {
  return <RpcContext.Provider value={rpc}>{children}</RpcContext.Provider>;
}

export function useRpc() {
  return useContext(RpcContext);
}

// Generic query hook
export function useRpcQuery<K extends RpcKey>(
  key: K,
  input: Parameters<RpcFunctions[K]>[0],
  options: { enabled?: boolean } = {}
): UseQueryResult<Awaited<ReturnType<RpcFunctions[K]>>> {
  type TOutput = Awaited<ReturnType<RpcFunctions[K]>>;
  
  const { enabled = true } = options;
  const [data, setData] = useState<TOutput>();
  const [error, setError] = useState<Error | null>(null);
  const [isLoading, setIsLoading] = useState(enabled);
  const mountedRef = useRef(true);
  const inputRef = useRef(input);
  inputRef.current = input;

  const fetchData = useCallback(async () => {
    if (!enabled) return;
    setIsLoading(true);
    setError(null);

    try {
      const fn = rpc[key] as (input: any) => Promise<TOutput>;
      const result = await fn(inputRef.current);
      if (mountedRef.current) {
        setData(result);
        setIsLoading(false);
      }
    } catch (err) {
      if (mountedRef.current) {
        setError(err instanceof Error ? err : new Error(String(err)));
        setIsLoading(false);
      }
    }
  }, [key, enabled]);

  useEffect(() => {
    fetchData();
  }, [fetchData, JSON.stringify(input)]);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  return {
    data,
    error,
    isLoading,
    isError: error !== null,
    isSuccess: data !== undefined && error === null,
    refetch: fetchData,
  };
}

// Generic mutation hook
export function useRpcMutation<K extends RpcKey>(
  key: K,
  options: {
    onSuccess?: (data: Awaited<ReturnType<RpcFunctions[K]>>, input: Parameters<RpcFunctions[K]>[0]) => void;
    onError?: (error: Error, input: Parameters<RpcFunctions[K]>[0]) => void;
  } = {}
): UseMutationResult<Parameters<RpcFunctions[K]>[0], Awaited<ReturnType<RpcFunctions[K]>>> {
  type TInput = Parameters<RpcFunctions[K]>[0];
  type TOutput = Awaited<ReturnType<RpcFunctions[K]>>;
  
  const { onSuccess, onError } = options;
  const [data, setData] = useState<TOutput>();
  const [error, setError] = useState<Error | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const mountedRef = useRef(true);

  const mutateAsync = useCallback(
    async (input: TInput): Promise<TOutput> => {
      setStatus('loading');
      setError(null);

      try {
        const fn = rpc[key] as (input: TInput) => Promise<TOutput>;
        const result = await fn(input);
        if (mountedRef.current) {
          setData(result);
          setStatus('success');
          onSuccess?.(result, input);
        }
        return result;
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        if (mountedRef.current) {
          setError(error);
          setStatus('error');
          onError?.(error, input);
        }
        throw error;
      }
    },
    [key, onSuccess, onError]
  );

  const mutate = useCallback(
    (input: TInput) => { mutateAsync(input).catch(() => {}); },
    [mutateAsync]
  );

  const reset = useCallback(() => {
    setData(undefined);
    setError(null);
    setStatus('idle');
  }, []);

  useEffect(() => {
    mountedRef.current = true;
    return () => { mountedRef.current = false; };
  }, []);

  return {
    data,
    error,
    isLoading: status === 'loading',
    isError: status === 'error',
    isSuccess: status === 'success',
    isIdle: status === 'idle',
    mutate,
    mutateAsync,
    reset,
  };
}

// Typed convenience hooks
export function useGreet(input: { name: string }, options?: { enabled?: boolean }) {
  return useRpcQuery('greet', input, options);
}

export function useGetUser(input: { id: number }, options?: { enabled?: boolean }) {
  return useRpcQuery('getUser', input, options);
}

export function useListUsers(input: Parameters<typeof rpc.listUsers>[0] = {}, options?: { enabled?: boolean }) {
  return useRpcQuery('listUsers', input, options);
}

export function useCreateUser(options?: Parameters<typeof useRpcMutation<'createUser'>>[1]) {
  return useRpcMutation('createUser', options);
}

export function useUpdateUser(options?: Parameters<typeof useRpcMutation<'updateUser'>>[1]) {
  return useRpcMutation('updateUser', options);
}

export function useDeleteUser(options?: Parameters<typeof useRpcMutation<'deleteUser'>>[1]) {
  return useRpcMutation('deleteUser', options);
}
