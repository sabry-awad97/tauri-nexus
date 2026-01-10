/**
 * React hooks for Tauri RPC
 */

import {
  createContext,
  useContext,
  useState,
  useEffect,
  useCallback,
  useRef,
  type ReactNode,
} from 'react';
import { createClient, type ClientConfig } from './client';
import type { RouterDef, ProcedureDef, InferInput, InferOutput } from './types';

export interface UseQueryResult<T> {
  data: T | undefined;
  error: Error | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  refetch: () => Promise<void>;
}

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

type ProcedureKeys<T extends RouterDef> = {
  [K in keyof T]: T[K] extends ProcedureDef<any, any> ? K : never;
}[keyof T];

export function createReactClient<T extends RouterDef>(
  routerDef: T,
  config: ClientConfig = {}
) {
  const client = createClient(routerDef, config);
  const ClientContext = createContext<typeof client>(client);

  function Provider({ children }: { children: ReactNode }) {
    return (
      <ClientContext.Provider value={client}>{children}</ClientContext.Provider>
    );
  }

  function useClient() {
    return useContext(ClientContext);
  }

  function useQuery<K extends ProcedureKeys<T>>(
    key: K,
    input: InferInput<T[K]>,
    options: { enabled?: boolean } = {}
  ): UseQueryResult<InferOutput<T[K]>> {
    const { enabled = true } = options;
    const [data, setData] = useState<InferOutput<T[K]>>();
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
        const proc = (client as any)[key];
        const result = await proc(inputRef.current);
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
      return () => {
        mountedRef.current = false;
      };
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

  function useMutation<K extends ProcedureKeys<T>>(
    key: K,
    options: {
      onSuccess?: (data: InferOutput<T[K]>, input: InferInput<T[K]>) => void;
      onError?: (error: Error, input: InferInput<T[K]>) => void;
    } = {}
  ): UseMutationResult<InferInput<T[K]>, InferOutput<T[K]>> {
    const { onSuccess, onError } = options;
    const [data, setData] = useState<InferOutput<T[K]>>();
    const [error, setError] = useState<Error | null>(null);
    const [status, setStatus] = useState<'idle' | 'loading' | 'success' | 'error'>('idle');
    const mountedRef = useRef(true);

    const mutateAsync = useCallback(
      async (input: InferInput<T[K]>): Promise<InferOutput<T[K]>> => {
        setStatus('loading');
        setError(null);

        try {
          const proc = (client as any)[key];
          const result = await proc(input);
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
      (input: InferInput<T[K]>) => {
        mutateAsync(input).catch(() => {});
      },
      [mutateAsync]
    );

    const reset = useCallback(() => {
      setData(undefined);
      setError(null);
      setStatus('idle');
    }, []);

    useEffect(() => {
      mountedRef.current = true;
      return () => {
        mountedRef.current = false;
      };
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

  return { client, Provider, useClient, useQuery, useMutation };
}
