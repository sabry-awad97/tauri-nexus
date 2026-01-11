// =============================================================================
// RPC Contract Definition
// =============================================================================
// This file sets up the RPC client and React integration.
// Types and contract are imported from the auto-generated Tauri bindings.

import {
  createClientFromSchema,
  createTanstackQueryUtils,
  useSubscription,
  isRpcError,
  hasErrorCode,
  getProcedures,
  type RpcError,
  type SubscriptionResult,
  type SubscriptionHookOptions,
} from "@tauri-nexus/rpc-react";
import {
  QueryClient,
  QueryClientProvider,
  useQueryClient,
} from "@tanstack/react-query";
import { createContext, useContext, type ReactNode } from "react";

// Import schema and types from auto-generated Tauri bindings
import {
  appContractSchema,
  type AppContract,
  type CounterInput,
  type CounterEvent,
  type ChatMessage,
  type StockPrice,
} from "../generated/bindings";

// =============================================================================
// Client Instance
// =============================================================================

export const rpc = createClientFromSchema(appContractSchema);

// =============================================================================
// TanStack Query Utils (oRPC-style API)
// =============================================================================

/**
 * TanStack Query utilities for the RPC client.
 *
 * @example
 * ```typescript
 * // Query
 * const { data } = useQuery(orpc.user.get.queryOptions({ input: { id: 1 } }));
 * const { data } = useQuery(orpc.health.queryOptions());
 *
 * // Mutation
 * const { mutate } = useMutation(orpc.user.create.mutationOptions());
 *
 * // Cache invalidation
 * queryClient.invalidateQueries({ queryKey: orpc.user.key() });
 *
 * // Direct call
 * const user = await orpc.user.get.call({ id: 1 });
 * ```
 */
export const orpc = createTanstackQueryUtils<AppContract>(rpc);

// =============================================================================
// Query Client
// =============================================================================

export const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60, // 1 minute
      retry: 1,
    },
  },
});

// =============================================================================
// RPC Provider
// =============================================================================

const RpcContext = createContext<typeof rpc>(rpc);

export function RpcProvider({ children }: { children: ReactNode }) {
  return (
    <QueryClientProvider client={queryClient}>
      <RpcContext.Provider value={rpc}>{children}</RpcContext.Provider>
    </QueryClientProvider>
  );
}

export function useRpc() {
  return useContext(RpcContext);
}

// =============================================================================
// Typed Subscription Hooks
// =============================================================================

/** Counter subscription */
export function useCounter(
  input: CounterInput = {},
  options?: SubscriptionHookOptions<CounterEvent>,
): SubscriptionResult<CounterEvent> {
  return useSubscription(
    () => rpc.stream.counter(input),
    [input.start, input.maxCount, input.intervalMs],
    options,
  );
}

/** Time subscription */
export function useTime(
  options?: SubscriptionHookOptions<string>,
): SubscriptionResult<string> {
  return useSubscription(() => rpc.stream.time(), [], options);
}

/** Stock subscription */
export function useStocks(
  symbols: string[],
  options?: SubscriptionHookOptions<StockPrice>,
): SubscriptionResult<StockPrice> {
  return useSubscription(
    () => rpc.stream.stocks({ symbols }),
    [symbols.join(",")],
    options,
  );
}

/** Chat subscription */
export function useChat(
  roomId: string,
  options?: SubscriptionHookOptions<ChatMessage>,
): SubscriptionResult<ChatMessage> {
  return useSubscription(() => rpc.stream.chat({ roomId }), [roomId], options);
}

// =============================================================================
// Namespace Exports
// =============================================================================

export const user = rpc.user;
export const stream = rpc.stream;
export const chat = rpc.chat;

// Re-export utilities
export {
  isRpcError,
  hasErrorCode,
  getProcedures,
  useSubscription,
  useQueryClient,
};
export type { RpcError, SubscriptionResult, SubscriptionHookOptions };

// Re-export types from bindings
export type {
  User,
  AppContract,
  CounterInput,
  CounterEvent,
  ChatMessage,
  StockPrice,
} from "../generated/bindings";
