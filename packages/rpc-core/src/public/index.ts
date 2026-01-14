// =============================================================================
// @tauri-nexus/rpc-core - Public API
// =============================================================================
// Consolidated Promise-based public API. Effect APIs are internal only.

// =============================================================================
// Core Types
// =============================================================================

export type {
  RpcErrorCode,
  RpcError,
  ProcedureType,
  ProcedureDef,
  QueryDef,
  MutationDef,
  SubscriptionDef,
  ProcedureDefinition,
  IsProcedure,
  IsRouter,
  Event,
  EventMeta,
  SingleRequest,
  BatchRequest,
  BatchResult,
  BatchResponse,
  CallOptions,
  SubscriptionOptions,
  BatchCallOptions,
  SubscribeRequest,
  RequestContext,
  ResponseContext,
  Middleware,
  EventIterator,
  DeepPartial,
  Prettify,
} from "../core/types";

// =============================================================================
// Inference Types
// =============================================================================

export type {
  InferInput,
  InferOutput,
  InferProcedureType,
  IsQuery,
  IsMutation,
  IsSubscription,
  ExtractPaths,
  ExtractSubscriptionPaths,
  GetProcedureAtPath,
  ExtractCallablePaths,
  GetInputAtPath,
  GetOutputAtPath,
  TypedBatchResult,
  BatchRequestEntry,
  ProcedureClient,
  RouterClient,
  InferClientInputs,
  InferClientOutputs,
  InferClientBodyInputs,
  InferClientBodyOutputs,
  InferClientErrors,
  InferClientErrorUnion,
  InferClientProcedureTypes,
  InferClientInputUnion,
  InferClientOutputUnion,
  InferClientContext,
} from "../core/inference";

// =============================================================================
// Error Utilities (from rpc-effect)
// =============================================================================

export {
  // Type guards
  isRpcError,
  hasErrorCode,
  // Constructors
  createRpcError,
  // Backward compatibility alias
  createRpcError as createError,
  // Rate limit
  isRateLimitError,
  getRateLimitRetryAfter,
} from "@tauri-nexus/rpc-effect";

// rpc-core specific error utilities
export { parseError } from "../core/errors";

// =============================================================================
// Validation
// =============================================================================

export { validatePath, isValidPath } from "./validation";

// =============================================================================
// Contract Builders
// =============================================================================

export { query, mutation, subscription } from "../core/contract";

// =============================================================================
// Client Configuration
// =============================================================================

export {
  configureRpc,
  getConfig,
  type RpcClientConfig,
} from "../client/config";

// =============================================================================
// Core Call Functions
// =============================================================================

export { call, subscribe, executeBatch } from "./call";

// =============================================================================
// Batch Operations
// =============================================================================

export {
  TypedBatchBuilder,
  TypedBatchResponse as TypedBatchResponseWrapper,
  TypedBatchResponse,
} from "./batch";

// =============================================================================
// Client Factories
// =============================================================================

export {
  createClient,
  createClientWithSubscriptions,
  type RpcClient,
} from "./factory";

// =============================================================================
// Subscription
// =============================================================================

export {
  createEventIterator,
  consumeEventIterator,
  type ConsumeOptions,
} from "./event-iterator";

// =============================================================================
// Link Types
// =============================================================================

export type {
  LinkRequestContext,
  LinkResponse,
  LinkInterceptor,
  ErrorHandler,
  RequestHandler,
  ResponseHandler,
  TauriLinkConfig,
  LinkCallOptions,
  LinkSubscribeOptions,
} from "../link/types";

// =============================================================================
// TauriLink
// =============================================================================

export { TauriLink } from "../link/tauri-link";

// =============================================================================
// Link Client Factory
// =============================================================================

export {
  createClientFromLink,
  type LinkRouterClient,
} from "../link/client-factory";

// =============================================================================
// Link Interceptors
// =============================================================================

export {
  onError,
  logging,
  retry,
  authInterceptor,
  type AuthInterceptorOptions,
} from "../link/interceptors";

// =============================================================================
// Link Type Inference
// =============================================================================

import type { TauriLink } from "../link/tauri-link";
import type { LinkRouterClient } from "../link/client-factory";

export type InferLinkContext<T> = T extends TauriLink<infer C> ? C : never;
export type InferLinkClientContext<T> =
  T extends LinkRouterClient<unknown, infer C> ? C : never;

// =============================================================================
// Utility Functions
// =============================================================================

export {
  // Timing
  sleep,
  calculateBackoff,
  // Retry
  withRetry,
  type RetryConfig,
  defaultRetryConfig,
  // Serialization
  stableStringify,
  deduplicationKey,
  // Deduplication
  withDedup,
  // Backend utilities
  getProcedures,
  getSubscriptionCount,
} from "./utils";

// =============================================================================
// Schema Types
// =============================================================================

export type {
  SchemaProcedure,
  SchemaContract,
  InferSchemaInput,
  InferSchemaOutput,
  InferSchemaProcedureType,
  InferContractInputs,
  InferContractOutputs,
  InferProcedureInput,
  InferProcedureOutput,
  SchemaContractToContract,
  ExtractEventsType,
  InferEventName,
  EventPayload,
  ExtractEventsOptions,
  ValidationErrorDetails,
  ValidationConfig,
} from "../schema/types";

// =============================================================================
// Schema Builder
// =============================================================================

export {
  ProcedureBuilder,
  procedure,
  router,
  mergeRouters,
} from "../schema/builder";

// =============================================================================
// Schema Validation
// =============================================================================

export {
  buildSchemaMap,
  createValidationInterceptor,
} from "../schema/validation";

// =============================================================================
// Schema Path Extraction
// =============================================================================

export {
  extractPaths,
  extractSubscriptionPaths,
  extractQueryPaths,
  extractMutationPaths,
  extractPathsByType,
  extractEvents,
} from "../schema/path-extraction";

// =============================================================================
// Schema Client Factories
// =============================================================================

export {
  createValidatedClient,
  createClientFromSchema,
  type SchemaClientConfig,
} from "../schema/client-factory";
