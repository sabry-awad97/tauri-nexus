// =============================================================================
// RPC Contract Definition
// =============================================================================
// Define your RPC contract here. The contract specifies all available
// procedures (queries, mutations, subscriptions) and their input/output types.
//
// This is the single source of truth for your RPC API types.
// The client will automatically infer all types from this contract.

import {
  createClientWithSubscriptions,
  useSubscription,
  isRpcError,
  hasErrorCode,
  getProcedures,
  type ContractRouter,
  type RpcError,
  type SubscriptionResult,
  type SubscriptionHookOptions,
} from "../lib/rpc";
import {
  QueryClient,
  QueryClientProvider,
  useQuery,
  useMutation,
  type UseQueryOptions,
  type UseMutationOptions,
} from "@tanstack/react-query";
import { createContext, useContext, type ReactNode } from "react";

// =============================================================================
// Domain Types
// =============================================================================

/** User entity */
export interface User {
  id: number;
  name: string;
  email: string;
  createdAt: string;
}

/** Input for creating a user */
export interface CreateUserInput {
  name: string;
  email: string;
}

/** Input for updating a user */
export interface UpdateUserInput {
  id: number;
  name?: string;
  email?: string;
}

/** Health check response */
export interface HealthResponse {
  status: string;
  version: string;
}

/** Generic success response */
export interface SuccessResponse {
  success: boolean;
  message?: string;
}

// =============================================================================
// Subscription Types
// =============================================================================

/** Counter subscription input */
export interface CounterInput {
  start?: number;
  maxCount?: number;
  intervalMs?: number;
}

/** Counter event */
export interface CounterEvent {
  count: number;
  timestamp: string;
}

/** Chat room subscription input */
export interface ChatRoomInput {
  roomId: string;
}

/** Chat message event */
export interface ChatMessage {
  id: string;
  roomId: string;
  userId: string;
  text: string;
  timestamp: string;
}

/** Send message input */
export interface SendMessageInput {
  roomId: string;
  text: string;
}

/** Stock subscription input */
export interface StockInput {
  symbols: string[];
}

/** Stock price event */
export interface StockPrice {
  symbol: string;
  price: number;
  change: number;
  changePercent: number;
  timestamp: string;
}

// =============================================================================
// Contract Definition
// =============================================================================

export interface AppContract extends ContractRouter {
  health: { type: "query"; input: void; output: HealthResponse };
  greet: { type: "query"; input: { name: string }; output: string };

  user: {
    get: { type: "query"; input: { id: number }; output: User };
    list: { type: "query"; input: void; output: User[] };
    create: { type: "mutation"; input: CreateUserInput; output: User };
    update: { type: "mutation"; input: UpdateUserInput; output: User };
    delete: { type: "mutation"; input: { id: number }; output: SuccessResponse };
  };

  stream: {
    counter: { type: "subscription"; input: CounterInput; output: CounterEvent };
    stocks: { type: "subscription"; input: StockInput; output: StockPrice };
    chat: { type: "subscription"; input: ChatRoomInput; output: ChatMessage };
    time: { type: "subscription"; input: void; output: string };
  };

  chat: {
    send: { type: "mutation"; input: SendMessageInput; output: ChatMessage };
    history: { type: "query"; input: { roomId: string; limit?: number }; output: ChatMessage[] };
  };
}

// =============================================================================
// Client Instance
// =============================================================================

const SUBSCRIPTION_PATHS = [
  "stream.counter",
  "stream.stocks",
  "stream.chat",
  "stream.time",
] as const;

export const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: [...SUBSCRIPTION_PATHS],
});

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
// Query Keys
// =============================================================================

export const queryKeys = {
  health: ["health"] as const,
  greet: (name: string) => ["greet", name] as const,
  user: {
    all: ["user"] as const,
    list: () => ["user", "list"] as const,
    detail: (id: number) => ["user", "detail", id] as const,
  },
  chat: {
    history: (roomId: string) => ["chat", "history", roomId] as const,
  },
} as const;

// =============================================================================
// Typed Query Hooks (using TanStack Query)
// =============================================================================

/** Health check */
export function useHealth(
  options?: Omit<UseQueryOptions<HealthResponse, RpcError>, "queryKey" | "queryFn">
) {
  return useQuery({
    queryKey: queryKeys.health,
    queryFn: () => rpc.health(),
    ...options,
  });
}

/** Greet */
export function useGreet(
  name: string,
  options?: Omit<UseQueryOptions<string, RpcError>, "queryKey" | "queryFn">
) {
  return useQuery({
    queryKey: queryKeys.greet(name),
    queryFn: () => rpc.greet({ name }),
    ...options,
  });
}

/** Get user by ID */
export function useUser(
  id: number,
  options?: Omit<UseQueryOptions<User, RpcError>, "queryKey" | "queryFn">
) {
  return useQuery({
    queryKey: queryKeys.user.detail(id),
    queryFn: () => rpc.user.get({ id }),
    enabled: id > 0,
    ...options,
  });
}

/** List all users */
export function useUsers(
  options?: Omit<UseQueryOptions<User[], RpcError>, "queryKey" | "queryFn">
) {
  return useQuery({
    queryKey: queryKeys.user.list(),
    queryFn: () => rpc.user.list(),
    ...options,
  });
}

// =============================================================================
// Typed Mutation Hooks (using TanStack Query)
// =============================================================================

/** Create user */
export function useCreateUser(
  options?: Omit<UseMutationOptions<User, RpcError, CreateUserInput>, "mutationFn">
) {
  return useMutation({
    mutationFn: (input: CreateUserInput) => rpc.user.create(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.user.all });
    },
    ...options,
  });
}

/** Update user */
export function useUpdateUser(
  options?: Omit<UseMutationOptions<User, RpcError, UpdateUserInput>, "mutationFn">
) {
  return useMutation({
    mutationFn: (input: UpdateUserInput) => rpc.user.update(input),
    onSuccess: (data) => {
      queryClient.invalidateQueries({ queryKey: queryKeys.user.detail(data.id) });
      queryClient.invalidateQueries({ queryKey: queryKeys.user.list() });
    },
    ...options,
  });
}

/** Delete user */
export function useDeleteUser(
  options?: Omit<UseMutationOptions<SuccessResponse, RpcError, { id: number }>, "mutationFn">
) {
  return useMutation({
    mutationFn: (input: { id: number }) => rpc.user.delete(input),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.user.all });
    },
    ...options,
  });
}

// =============================================================================
// Typed Subscription Hooks
// =============================================================================

/** Counter subscription */
export function useCounter(
  input: CounterInput = {},
  options?: SubscriptionHookOptions<CounterEvent>
): SubscriptionResult<CounterEvent> {
  return useSubscription(
    () => rpc.stream.counter(input),
    [input.start, input.maxCount, input.intervalMs],
    options
  );
}

/** Time subscription */
export function useTime(
  options?: SubscriptionHookOptions<string>
): SubscriptionResult<string> {
  return useSubscription(() => rpc.stream.time(), [], options);
}

/** Stock subscription */
export function useStocks(
  symbols: string[],
  options?: SubscriptionHookOptions<StockPrice>
): SubscriptionResult<StockPrice> {
  return useSubscription(
    () => rpc.stream.stocks({ symbols }),
    [symbols.join(",")],
    options
  );
}

/** Chat subscription */
export function useChat(
  roomId: string,
  options?: SubscriptionHookOptions<ChatMessage>
): SubscriptionResult<ChatMessage> {
  return useSubscription(
    () => rpc.stream.chat({ roomId }),
    [roomId],
    options
  );
}

// =============================================================================
// Namespace Exports
// =============================================================================

export const user = rpc.user;
export const stream = rpc.stream;
export const chat = rpc.chat;

// Re-export utilities
export { isRpcError, hasErrorCode, getProcedures, useSubscription };
export type { RpcError, SubscriptionResult, SubscriptionHookOptions };
