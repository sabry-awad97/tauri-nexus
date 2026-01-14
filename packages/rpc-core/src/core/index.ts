// =============================================================================
// @tauri-nexus/rpc-core - Core Module
// =============================================================================

// Types
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
} from "./types";

// Inference types
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
} from "./inference";

// Error utilities
export {
  isRpcError,
  hasErrorCode,
  createError,
  parseError,
  isRateLimitError,
  getRateLimitRetryAfter,
} from "./errors";

// Validation (re-exported from rpc-effect)
export {
  validatePathEffect,
  validatePathsEffect,
  validateAndNormalizePathEffect,
  isValidPathEffect,
  validatePathWithRulesEffect,
  type PathValidationRules,
} from "./validation";

// Contract builders
export { query, mutation, subscription } from "./contract";
