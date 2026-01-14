// =============================================================================
// Core Module Exports
// =============================================================================

// Types
export type {
  ProcedureType,
  Event,
  EventIterator,
  ValidationIssue,
  RpcConfig,
  RpcTransport,
  SubscribeTransportOptions,
  RpcInterceptorChain,
  RpcInterceptor,
  InterceptorContext,
  RpcLogger,
  EffectRequestContext,
  EffectResponseContext,
} from "./types";

// Error classes
export {
  RpcCallError,
  RpcTimeoutError,
  RpcCancelledError,
  RpcValidationError,
  RpcNetworkError,
  type RpcEffectError,
} from "./errors";

// Error utilities
export {
  // Constructors
  createCallError,
  createTimeoutError,
  createCancelledError,
  createValidationError,
  createNetworkError,
  // Type guards
  isEffectRpcError,
  isRpcCallError,
  isRpcTimeoutError,
  isRpcCancelledError,
  isRpcValidationError,
  isRpcNetworkError,
  // Code utilities
  type VirtualErrorCode,
  getErrorCode,
  hasCode,
  hasAnyCode,
  isRetryableError,
  // Pattern matching
  type ErrorHandlers,
  matchError,
  // Effect combinators
  failWithCallError,
  failWithTimeout,
  failWithValidation,
  failWithNetwork,
  failWithCancelled,
} from "./error-utils";
