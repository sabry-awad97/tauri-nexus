// =============================================================================
// @tauri-nexus/rpc-core - Error Handling
// =============================================================================
// Pure re-exports from rpc-effect (single source of truth).
// NO wrappers, NO aliases - just re-exports.

export {
  // Public Error Types
  type PublicRpcError,
  type PublicRpcError as RpcError,
  type RpcErrorCode,
  // Public Error Utilities
  toPublicError,
  fromPublicError,
  isPublicRpcError,
  isPublicRpcError as isRpcError,
  hasPublicErrorCode,
  hasPublicErrorCode as hasErrorCode,
  createPublicError,
  createPublicError as createError,
  // Rate Limit Helpers
  isRateLimitError,
  getRateLimitRetryAfter,
  // Error Parsing
  type RpcErrorShape,
  type ErrorParserOptions,
  isRpcErrorShape,
  parseJsonError,
  makeCallErrorFromShape,
  parseToEffectError,
  fromTransportError,
  parseEffectError,
  parseToPublicError,
  parseToPublicError as parseError,
  // Effect Error Constructors
  makeCallError,
  makeTimeoutError,
  makeCancelledError,
  makeValidationError,
  makeNetworkError,
  // Effect Type Guards
  isEffectRpcError,
} from "@tauri-nexus/rpc-effect";
