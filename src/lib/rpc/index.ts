// =============================================================================
// Tauri RPC Client Library
// =============================================================================
// A fully type-safe RPC client library for Tauri v2 applications.
//
// Features:
// - Contract-first design: define types once, get full type safety everywhere
// - Proxy-based client: automatic path generation from contract structure
// - Subscriptions: async iterators with auto-reconnect support
// - React hooks: useQuery, useMutation, useSubscription with full type inference
// - Middleware: extensible request/response pipeline
// - Error handling: typed errors with error codes
//
// Quick Start:
// ```typescript
// // 1. Define your contract
// interface MyContract extends ContractRouter {
//   health: { type: 'query'; input: void; output: { status: string } };
//   user: {
//     get: { type: 'query'; input: { id: number }; output: User };
//     create: { type: 'mutation'; input: CreateUserInput; output: User };
//   };
//   stream: {
//     events: { type: 'subscription'; input: void; output: Event };
//   };
// }
//
// // 2. Create a typed client
// const rpc = createClient<MyContract>({
//   subscriptionPaths: ['stream.events'],
// });
//
// // 3. Use with full type safety!
// const health = await rpc.health();
// const user = await rpc.user.get({ id: 1 });
// for await (const event of await rpc.stream.events()) {
//   console.log(event);
// }
// ```

// =============================================================================
// Core Types
// =============================================================================

export type {
  // Error types
  RpcError,
  RpcErrorCode,
  // Event types
  Event,
  EventMeta,
  // Procedure definition types
  ProcedureType,
  ProcedureDef,
  QueryDef,
  MutationDef,
  SubscriptionDef,
  // Contract types
  ContractRouter,
  IsProcedure,
  IsRouter,
  // Type inference utilities
  InferInput,
  InferOutput,
  InferProcedureType,
  IsQuery,
  IsMutation,
  IsSubscription,
  // Client types
  EventIterator,
  CallOptions,
  SubscriptionOptions,
  ProcedureClient,
  RouterClient,
  // Path extraction types
  ExtractPaths,
  ExtractSubscriptionPaths,
  GetProcedureAtPath,
  // Middleware types
  RequestContext,
  ResponseContext,
  Middleware,
  // Request types
  SubscribeRequest,
  // Utility types
  DeepPartial,
  Prettify,
  // Client inference utilities
  InferClientInputs,
  InferClientOutputs,
  InferClientBodyInputs,
  InferClientBodyOutputs,
  InferClientErrors,
  InferClientErrorUnion,
  InferClientProcedureTypes,
  InferClientInputUnion,
  InferClientOutputUnion,
  InferClientContext as InferContractClientContext,
} from "./types";

// Contract builder helpers
export { query, mutation, subscription } from "./types";

// =============================================================================
// Client
// =============================================================================

export {
  // Client factories
  createClient,
  createClientWithSubscriptions,
  // Configuration
  configureRpc,
  getConfig,
  // Core functions
  call,
  subscribe,
  // Validation
  validatePath,
  // Error utilities
  isRpcError,
  hasErrorCode,
  createError,
  // Backend utilities
  getProcedures,
  getSubscriptionCount,
  // Types
  type RpcClientConfig,
} from "./client";

// =============================================================================
// Event Iterator
// =============================================================================

export {
  createEventIterator,
  consumeEventIterator,
  type ConsumeOptions,
} from "./event-iterator";

// =============================================================================
// React Hooks (Subscription only - use TanStack Query for queries/mutations)
// =============================================================================

export {
  // Subscription hook (TanStack Query doesn't support streaming)
  useSubscription,
  // Utility hooks
  useIsMounted,
  // Types
  type SubscriptionState,
  type SubscriptionResult,
  type SubscriptionHookOptions,
} from "./hooks";

// =============================================================================
// TanStack Query Integration
// =============================================================================

export {
  createTanstackQueryUtils,
  type TanstackQueryUtils,
  type CreateTanstackQueryUtilsOptions,
  type QueryOptionsResult,
  type MutationOptionsResult,
  type InfiniteOptionsResult,
  type KeyOptions,
} from "./tanstack";

// =============================================================================
// TauriLink (oRPC-style Link Abstraction)
// =============================================================================

export {
  TauriLink,
  createClientFromLink,
  // Interceptor helpers
  onError,
  logging,
  retry,
  // Types
  type TauriLinkConfig,
  type LinkRequestContext,
  type LinkResponse,
  type LinkInterceptor,
  type LinkCallOptions,
  type LinkSubscribeOptions,
  type LinkRouterClient,
  type ErrorHandler,
  type RequestHandler,
  type ResponseHandler,
  type InferLinkContext,
  type InferClientContext,
} from "./link";

// =============================================================================
// Utilities
// =============================================================================

export {
  getProcedures as listProcedures,
  sleep,
  calculateBackoff,
  withRetry,
  withDedup,
  deduplicationKey,
  stableStringify,
  defaultRetryConfig,
  type RetryConfig,
} from "./utils";
