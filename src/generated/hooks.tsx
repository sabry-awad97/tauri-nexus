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
} from "react";
import { rpc, user } from "./router";
import type {
  User,
  CreateUserInput,
  UpdateUserInput,
  GreetInput,
  RpcError,
} from "./types";
import { isRpcError } from "./client";

// -----------------------------------------------------------------------------
// Types
// -----------------------------------------------------------------------------

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

export interface MutationState<T> {
  data: T | undefined;
  error: RpcError | null;
  isLoading: boolean;
  isError: boolean;
  isSuccess: boolean;
  isIdle: boolean;
}

export interface MutationResult<
  TInput,
  TOutput,
> extends MutationState<TOutput> {
  mutate: (input: TInput) => void;
  mutateAsync: (input: TInput) => Promise<TOutput>;
  reset: () => void;
}

export interface QueryOptions {
  enabled?: boolean;
  refetchInterval?: number;
}

export interface MutationOptions<TInput, TOutput> {
  onSuccess?: (data: TOutput, input: TInput) => void;
  onError?: (error: RpcError, input: TInput) => void;
  onSettled?: () => void;
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
// Core Hooks
// -----------------------------------------------------------------------------

function useQuery<T>(
  queryFn: () => Promise<T>,
  deps: unknown[],
  options: QueryOptions = {},
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

    setState((s) => ({ ...s, isLoading: true, error: null }));

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
        const error = isRpcError(err)
          ? err
          : { code: "UNKNOWN", message: String(err) };
        setState((s) => ({
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

  // Refetch interval
  useEffect(() => {
    if (!refetchInterval || !enabled) return;
    const interval = setInterval(fetchData, refetchInterval);
    return () => clearInterval(interval);
  }, [refetchInterval, enabled, fetchData]);

  useEffect(() => {
    mountedRef.current = true;
    return () => {
      mountedRef.current = false;
    };
  }, []);

  return { ...state, refetch: fetchData };
}

function useMutation<TInput, TOutput>(
  mutationFn: (input: TInput) => Promise<TOutput>,
  options: MutationOptions<TInput, TOutput> = {},
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
      setState((s) => ({ ...s, isLoading: true, isIdle: false, error: null }));

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
        const error = isRpcError(err)
          ? err
          : { code: "UNKNOWN", message: String(err) };
        if (mountedRef.current) {
          setState((s) => ({
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
    [mutationFn, onSuccess, onError, onSettled],
  );

  const mutate = useCallback(
    (input: TInput) => {
      mutateAsync(input).catch(() => {});
    },
    [mutateAsync],
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

// -----------------------------------------------------------------------------
// Typed Hooks
// -----------------------------------------------------------------------------

// Health
export function useHealth(options?: QueryOptions) {
  return useQuery(() => rpc.health(), [], options);
}

// Greet
export function useGreet(input: GreetInput, options?: QueryOptions) {
  return useQuery(() => rpc.greet(input), [input.name], options);
}

// User queries
export function useUser(id: number, options?: QueryOptions) {
  return useQuery(() => user.get({ id }), [id], options);
}

export function useUsers(options?: QueryOptions) {
  return useQuery(() => user.list(), [], options);
}

// User mutations
export function useCreateUser(
  options?: MutationOptions<CreateUserInput, User>,
) {
  return useMutation((input: CreateUserInput) => user.create(input), options);
}

export function useUpdateUser(
  options?: MutationOptions<UpdateUserInput, User>,
) {
  return useMutation((input: UpdateUserInput) => user.update(input), options);
}

export function useDeleteUser(
  options?: MutationOptions<{ id: number }, unknown>,
) {
  return useMutation((input: { id: number }) => user.delete(input), options);
}
