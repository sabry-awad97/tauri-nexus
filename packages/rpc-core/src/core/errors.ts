// =============================================================================
// @tauri-nexus/rpc-core - Error Types
// =============================================================================
// Promise-based error handling. Types from rpc-effect, utilities for Promise API.

import {
  type RpcEffectError,
  toPublicError,
  parseEffectError,
  isRateLimitError as isRateLimitErrorEffect,
  getRateLimitRetryAfter as getRateLimitRetryAfterEffect,
} from "@tauri-nexus/rpc-effect";

// =============================================================================
// Types
// =============================================================================

/**
 * RPC error for Promise-based API.
 */
export interface RpcError {
  readonly code: string;
  readonly message: string;
  readonly details?: unknown;
  readonly cause?: string;
}

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

// =============================================================================
// Type Guards
// =============================================================================

export const isRpcError = (error: unknown): error is RpcError =>
  typeof error === "object" &&
  error !== null &&
  "code" in error &&
  "message" in error &&
  typeof (error as RpcError).code === "string" &&
  typeof (error as RpcError).message === "string";

export const hasErrorCode = (
  error: unknown,
  code: RpcErrorCode | string
): boolean => isRpcError(error) && error.code === code;

// =============================================================================
// Constructors
// =============================================================================

export const createError = (
  code: RpcErrorCode | string,
  message: string,
  details?: unknown
): RpcError => ({ code, message, details });

// =============================================================================
// Rate Limit
// =============================================================================

export const isRateLimitError = isRateLimitErrorEffect;
export const getRateLimitRetryAfter = getRateLimitRetryAfterEffect;

// =============================================================================
// Conversion
// =============================================================================

/**
 * Convert Effect error to RpcError.
 */
export const fromEffectError = (error: RpcEffectError): RpcError =>
  toPublicError(error);

/**
 * Parse any error to RpcError.
 */
export const parseError = (error: unknown, path: string = ""): RpcError =>
  toPublicError(parseEffectError(error, path));
