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
  createHooks,
  useQuery,
  useMutation,
  isRpcError,
  hasErrorCode,
  getProcedures,
  type ContractRouter,
  type RpcError,
  type QueryResult,
  type MutationResult,
  type QueryOptions,
  type MutationOptions,
} from "../lib/rpc";
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
// Define your RPC contract as a TypeScript interface.
// Each procedure specifies its type, input, and output.
//
// Procedure types:
// - query: Read-only operations (GET-like)
// - mutation: Write operations (POST/PUT/DELETE-like)
// - subscription: Streaming operations (WebSocket-like)

export interface AppContract extends ContractRouter {
  // Root-level procedures
  health: { type: "query"; input: void; output: HealthResponse };
  greet: { type: "query"; input: { name: string }; output: string };

  // User namespace - CRUD operations
  user: {
    get: { type: "query"; input: { id: number }; output: User };
    list: { type: "query"; input: void; output: User[] };
    create: { type: "mutation"; input: CreateUserInput; output: User };
    update: { type: "mutation"; input: UpdateUserInput; output: User };
    delete: {
      type: "mutation";
      input: { id: number };
      output: SuccessResponse;
    };
  };

  // Stream namespace - real-time subscriptions
  stream: {
    counter: {
      type: "subscription";
      input: CounterInput;
      output: CounterEvent;
    };
    stocks: { type: "subscription"; input: StockInput; output: StockPrice };
    chat: { type: "subscription"; input: ChatRoomInput; output: ChatMessage };
    time: { type: "subscription"; input: void; output: string };
  };

  // Chat namespace - chat operations
  chat: {
    send: { type: "mutation"; input: SendMessageInput; output: ChatMessage };
    history: {
      type: "query";
      input: { roomId: string; limit?: number };
      output: ChatMessage[];
    };
  };
}

// =============================================================================
// Subscription Paths
// =============================================================================
// List all subscription paths for runtime detection.
// This is required because TypeScript types are erased at runtime.

const SUBSCRIPTION_PATHS = [
  "stream.counter",
  "stream.stocks",
  "stream.chat",
  "stream.time",
] as const;

// =============================================================================
// Client Instance
// =============================================================================
// Create a typed client instance. This is the main export for your app.

export const rpc = createClientWithSubscriptions<AppContract>({
  subscriptionPaths: [...SUBSCRIPTION_PATHS],
});

// =============================================================================
// React Hooks
// =============================================================================
// Create typed React hooks bound to the client.

export const { useRpcQuery, useRpcMutation, useRpcSubscription } =
  createHooks(rpc);

// =============================================================================
// RPC Provider
// =============================================================================

const RpcContext = createContext<typeof rpc>(rpc);

export function RpcProvider({ children }: { children: ReactNode }) {
  return <RpcContext.Provider value={rpc}>{children}</RpcContext.Provider>;
}

export function useRpc() {
  return useContext(RpcContext);
}

// =============================================================================
// Typed Query Hooks
// =============================================================================

/** Health check hook */
export function useHealth(options?: QueryOptions) {
  return useQuery(() => rpc.health(), [], options);
}

/** Greet hook */
export function useGreet(input: { name: string }, options?: QueryOptions) {
  return useQuery(() => rpc.greet(input), [input.name], options);
}

/** Get user by ID hook */
export function useUser(id: number, options?: QueryOptions) {
  return useQuery(() => rpc.user.get({ id }), [id], options);
}

/** List all users hook */
export function useUsers(options?: QueryOptions) {
  return useQuery(() => rpc.user.list(), [], options);
}

// =============================================================================
// Typed Mutation Hooks
// =============================================================================

/** Create user mutation hook */
export function useCreateUser(options?: MutationOptions<CreateUserInput, User>) {
  return useMutation((input: CreateUserInput) => rpc.user.create(input), options);
}

/** Update user mutation hook */
export function useUpdateUser(options?: MutationOptions<UpdateUserInput, User>) {
  return useMutation((input: UpdateUserInput) => rpc.user.update(input), options);
}

/** Delete user mutation hook */
export function useDeleteUser(options?: MutationOptions<{ id: number }, SuccessResponse>) {
  return useMutation((input: { id: number }) => rpc.user.delete(input), options);
}

// =============================================================================
// Namespace Exports
// =============================================================================
// Export namespaces for convenient access.

export const user = rpc.user;
export const stream = rpc.stream;
export const chat = rpc.chat;

// Re-export utilities
export { isRpcError, hasErrorCode, getProcedures };
export type { RpcError, QueryResult, MutationResult, QueryOptions, MutationOptions };
