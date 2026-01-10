// =============================================================================
// Tauri RPC Client Library
// =============================================================================
// Type-safe RPC client with Event Iterator support for Tauri v2
// 
// Usage:
// 1. Define your contract types
// 2. Create a typed client with `createRpcClient<YourContract>()`
// 3. Everything is type-safe automatically!

// Core types
export type {
  RpcError,
  Event,
  SubscriptionOptions,
  CallOptions,
  ProcedureType,
  ProcedureDef,
  QueryDef,
  MutationDef,
  SubscriptionDef,
  ContractRouter,
  InferInput,
  InferOutput,
  InferProcedureType,
  IsSubscription,
  EventIterator,
  ProcedureClient,
  RouterClient,
} from './types';

// Contract builder helpers
export { query, mutation, subscription } from './types';

// Client
export {
  createRpcClient,
  createTypedClient,
  createClientWithSubscriptions,
  configureRpc,
  call,
  subscribe,
  isRpcError,
  hasErrorCode,
  type RpcClientConfig,
} from './client';

// Event iterator
export { createEventIterator, consumeEventIterator, type ConsumeOptions } from './event-iterator';

// React hooks
export {
  useQuery,
  useMutation,
  useSubscription,
  createHooks,
  type QueryState,
  type QueryResult,
  type QueryOptions,
  type MutationState,
  type MutationResult,
  type MutationOptions,
  type SubscriptionState,
  type SubscriptionResult,
  type SubscriptionOptions as HookSubscriptionOptions,
} from './hooks';

// Utilities
export {
  getProcedures,
  sleep,
  calculateBackoff,
  withRetry,
  withDedup,
  deduplicationKey,
  defaultRetryConfig,
  type RetryConfig,
} from './utils';
