// =============================================================================
// React Hooks for RPC
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
import { rpc, user } from './router';
import type { User, CreateUserInput, UpdateUserInput, GreetInput } from './types';

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

export interface QueryResult<T> {
  data: T | undefined;
  error: Error | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  refetch: () => Promise<void>;
}

export interface MutationResult<TInput, TOutput> {
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

export interface MutationOptions<TInput, TOutput> {
  onSuccess?: (data: TOutput, input: TInput) => void;
  onError?: (error: Error, input: TInput) => void;
}

// -----------------------------------------------------------------------------
// Context
// -----------------------------------------------------------------------------

const RpcContext = createContext<typeof rpc>(rpc);

export function RpcProvider({ children }: { children: ReactNode }) {
  return <RpcContext.Provider value={rpc}>{children}</RpcContext.Provider>;
}

export function useRpc() {
  return useContext(RpcContext);
}

// -----------------------------------------------------------------------------
// Generic Hooks
// -----------------------------------------------------------------------------

function useQuery<T>(
  queryFn: () => Promise<T>,
  deps: unknown[],
  options: { enabled?: boolean } = {}
): QueryResult<T> {
  const { enabled = true } = options;
  const [data, setData] = useState<T>();
  const [error, setError] = useState<Error | null>(null);
  const [isLoading, setIsLoading] = useState(enabled);
  const mountedRef = useRef(true);

  const fetchData = useCallback(async () => {
    if (!enabled) return;
    setIsLoading(true);
    setError(null);

    try {
      const result = await queryFn();
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
  // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [enabled, ...deps]);

  useEffect(() => {
    fetchData();
  }, [fetchData]);

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

function useMutation<TInput, TOutput>(
  mutationFn: (input: TInput) => Promise<TOutput>,
  options: MutationOptions<TInput, TOutput> = {}
): MutationResult<TInput, TOutput> {
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
        const result = await mutationFn(input);
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
    [mutationFn, onSuccess, onError]
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

// -----------------------------------------------------------------------------
// Typed Hooks
// -----------------------------------------------------------------------------

export function useGreet(input: GreetInput, options?: { enabled?: boolean }) {
  return useQuery(() => rpc.greet(input), [input.name], options);
}

export function useGetUser(input: { id: number }, options?: { enabled?: boolean }) {
  return useQuery(() => user.get(input), [input.id], options);
}

export function useListUsers(options?: { enabled?: boolean }) {
  return useQuery(() => user.list(), [], options);
}

export function useCreateUser(options?: MutationOptions<{ input: CreateUserInput }, User>) {
  return useMutation((args: { input: CreateUserInput }) => user.create(args.input), options);
}

export function useUpdateUser(options?: MutationOptions<{ input: UpdateUserInput }, User>) {
  return useMutation((args: { input: UpdateUserInput }) => user.update(args.input), options);
}

export function useDeleteUser(options?: MutationOptions<{ id: number }, unknown>) {
  return useMutation((args: { id: number }) => user.delete(args), options);
}
