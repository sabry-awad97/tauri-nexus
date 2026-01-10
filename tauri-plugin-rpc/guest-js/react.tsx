// Tauri RPC Plugin - React Hooks
// Dynamically typed based on command registry

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  useMemo,
  type ReactNode,
} from 'react';
import { rpc, type RpcCommands, type CommandName, type CommandInput, type CommandOutput } from './index';

// ============================================
// Types
// ============================================

export interface QueryOptions {
  enabled?: boolean;
  refetchInterval?: number;
  refetchOnWindowFocus?: boolean;
}

export interface QueryResult<T> {
  data: T | undefined;
  error: Error | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  refetch: () => Promise<void>;
}

export interface MutationOptions<TInput, TOutput> {
  onSuccess?: (data: TOutput, input: TInput) => void;
  onError?: (error: Error, input: TInput) => void;
  onSettled?: (data: TOutput | undefined, error: Error | null, input: TInput) => void;
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

// ============================================
// Context
// ============================================

const RpcContext = createContext<typeof rpc>(rpc);

export function RpcProvider({ children }: { children: ReactNode }) {
  return <RpcContext.Provider value={rpc}>{children}</RpcContext.Provider>;
}

export function useRpc() {
  return useContext(RpcContext);
}

// ============================================
// Generic Hooks
// ============================================

/**
 * Generic query hook - works with any RPC command
 */
export function useRpcQuery<K extends CommandName>(
  command: K,
  input: CommandInput<K>,
  options: QueryOptions = {}
): QueryResult<CommandOutput<K>> {
  const { enabled = true, refetchInterval, refetchOnWindowFocus = false } = options;
  
  const [data, setData] = useState<CommandOutput<K>>();
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
      const fn = rpc[command] as (input: CommandInput<K>) => Promise<CommandOutput<K>>;
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
  }, [command, enabled]);

  // Initial fetch and refetch on input change
  useEffect(() => {
    fetchData();
  }, [fetchData, JSON.stringify(input)]);

  // Refetch interval
  useEffect(() => {
    if (!refetchInterval || !enabled) return;
    const interval = setInterval(fetchData, refetchInterval);
    return () => clearInterval(interval);
  }, [refetchInterval, enabled, fetchData]);

  // Refetch on window focus
  useEffect(() => {
    if (!refetchOnWindowFocus || !enabled) return;
    const handleFocus = () => fetchData();
    window.addEventListener('focus', handleFocus);
    return () => window.removeEventListener('focus', handleFocus);
  }, [refetchOnWindowFocus, enabled, fetchData]);

  // Cleanup
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

/**
 * Generic mutation hook - works with any RPC command
 */
export function useRpcMutation<K extends CommandName>(
  command: K,
  options: MutationOptions<CommandInput<K>, CommandOutput<K>> = {}
): MutationResult<CommandInput<K>, CommandOutput<K>> {
  const { onSuccess, onError, onSettled } = options;
  
  const [data, setData] = useState<CommandOutput<K>>();
  const [error, setError] = useState<Error | null>(null);
  const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
  const mountedRef = useRef(true);

  const mutateAsync = useCallback(
    async (input: CommandInput<K>): Promise<CommandOutput<K>> => {
      setStatus('loading');
      setError(null);

      try {
        const fn = rpc[command] as (input: CommandInput<K>) => Promise<CommandOutput<K>>;
        const result = await fn(input);
        if (mountedRef.current) {
          setData(result);
          setStatus('success');
          onSuccess?.(result, input);
          onSettled?.(result, null, input);
        }
        return result;
      } catch (err) {
        const error = err instanceof Error ? err : new Error(String(err));
        if (mountedRef.current) {
          setError(error);
          setStatus('error');
          onError?.(error, input);
          onSettled?.(undefined, error, input);
        }
        throw error;
      }
    },
    [command, onSuccess, onError, onSettled]
  );

  const mutate = useCallback(
    (input: CommandInput<K>) => { mutateAsync(input).catch(() => {}); },
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

// ============================================
// Typed Convenience Hooks (auto-generated pattern)
// ============================================

// These are generated based on the command registry
// When you add a new command to Rust, add a hook here

export const useGreet = (input: CommandInput<'greet'>, options?: QueryOptions) =>
  useRpcQuery('greet', input, options);

export const useGetUser = (input: CommandInput<'getUser'>, options?: QueryOptions) =>
  useRpcQuery('getUser', input, options);

export const useListUsers = (input: CommandInput<'listUsers'> = {}, options?: QueryOptions) =>
  useRpcQuery('listUsers', input, options);

export const useCreateUser = (options?: MutationOptions<CommandInput<'createUser'>, CommandOutput<'createUser'>>) =>
  useRpcMutation('createUser', options);

export const useUpdateUser = (options?: MutationOptions<CommandInput<'updateUser'>, CommandOutput<'updateUser'>>) =>
  useRpcMutation('updateUser', options);

export const useDeleteUser = (options?: MutationOptions<CommandInput<'deleteUser'>, CommandOutput<'deleteUser'>>) =>
  useRpcMutation('deleteUser', options);

// ============================================
// Dynamic Hook Factory
// ============================================

/**
 * Create all hooks dynamically from the command registry
 * Usage: const { useGreet, useListUsers } = createRpcHooks();
 */
export function createRpcHooks() {
  type QueryHooks = {
    [K in CommandName as `use${Capitalize<K>}`]: (
      input: CommandInput<K>,
      options?: QueryOptions
    ) => QueryResult<CommandOutput<K>>;
  };

  type MutationHooks = {
    [K in CommandName as `use${Capitalize<K>}Mutation`]: (
      options?: MutationOptions<CommandInput<K>, CommandOutput<K>>
    ) => MutationResult<CommandInput<K>, CommandOutput<K>>;
  };

  const hooks = {} as QueryHooks & MutationHooks;

  for (const key of Object.keys(rpc) as CommandName[]) {
    const capitalizedKey = key.charAt(0).toUpperCase() + key.slice(1);
    
    // Query hook
    (hooks as any)[`use${capitalizedKey}`] = (input: any, options?: QueryOptions) =>
      useRpcQuery(key, input, options);
    
    // Mutation hook
    (hooks as any)[`use${capitalizedKey}Mutation`] = (options?: any) =>
      useRpcMutation(key, options);
  }

  return hooks;
}
