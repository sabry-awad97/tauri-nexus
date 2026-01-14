// =============================================================================
// Serializable RPC Error Types
// =============================================================================
// Plain serializable error types for Promise API and transport.

/**
 * Plain serializable RPC error object.
 * Used by Promise-based APIs and for transport/serialization.
 * Effect users should use RpcEffectError types for pattern matching.
 */
export interface RpcError {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}

/** Standard RPC error codes */
export type RpcErrorCode =
  | "BAD_REQUEST"
  | "UNAUTHORIZED"
  | "FORBIDDEN"
  | "NOT_FOUND"
  | "VALIDATION_ERROR"
  | "CONFLICT"
  | "PAYLOAD_TOO_LARGE"
  | "RATE_LIMITED"
  | "INTERNAL_ERROR"
  | "NOT_IMPLEMENTED"
  | "SERVICE_UNAVAILABLE"
  | "PROCEDURE_NOT_FOUND"
  | "SUBSCRIPTION_ERROR"
  | "MIDDLEWARE_ERROR"
  | "SERIALIZATION_ERROR"
  | "TIMEOUT"
  | "CANCELLED"
  | "UNKNOWN";

/** Shape for parsing errors from transport */
export interface RpcErrorShape {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}
