// =============================================================================
// @tauri-nexus/rpc-core - Vanilla TypeScript RPC Client
// =============================================================================
// Core RPC client library for Tauri v2 applications.
// No React dependencies - works with any framework.

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
  // Batch types
  SingleRequest,
  BatchRequest,
  BatchResult,
  BatchResponse,
  BatchCallOptions,
  // Type-safe batch types
  ExtractCallablePaths,
  GetInputAtPath,
  GetOutputAtPath,
  TypedSingleRequest,
  TypedBatchResult,
  BatchRequestEntry,
  // Procedure definition types
  ProcedureType,
  ProcedureDef,
  QueryDef,
  MutationDef,
  SubscriptionDef,
  // Contract types
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
  // Type-safe batch
  TypedBatchBuilder,
  TypedBatchResponseWrapper,
  TypedBatchResponseWrapper as TypedBatchResponse,
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
  type RpcClient,
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

// =============================================================================
// Zod Schema Validation
// =============================================================================

export {
  // Contract builder
  procedure,
  router,
  mergeRouters,
  ProcedureBuilder,
  // Client factory
  createClientFromSchema,
  // Validation
  createValidationInterceptor,
  createValidatedClient,
  buildSchemaMap,
  // Path extraction utilities
  extractPaths,
  extractSubscriptionPaths,
  extractQueryPaths,
  extractMutationPaths,
  extractPathsByType,
  // Event extraction utilities
  extractEvents,
  // Types
  type SchemaProcedure,
  type SchemaContract,
  type SchemaClientConfig,
  type ValidationConfig,
  type ValidationErrorDetails,
  type SchemaContractToContract,
  type ExtractEventsOptions,
  type ExtractEventsType,
  type InferEventName,
  type EventPayload,
  // Type inference utilities
  type InferSchemaInput,
  type InferSchemaOutput,
  type InferSchemaProcedureType,
  type InferContractInputs,
  type InferContractOutputs,
  type InferProcedureInput,
  type InferProcedureOutput,
} from "./schema";
