// =============================================================================
// Error Parsing
// =============================================================================
// Parse various error formats into Effect errors or serializable RpcError.

import type { RpcEffectError } from "../core/errors";
import {
  createCallError,
  createTimeoutError,
  createCancelledError,
  isEffectRpcError,
} from "../core/error-utils";
import type { RpcErrorShape, RpcError } from "./types";
import { toRpcError } from "./conversion";

// =============================================================================
// Shape Detection
// =============================================================================

export const isRpcErrorShape = (value: unknown): value is RpcErrorShape =>
  typeof value === "object" &&
  value !== null &&
  "code" in value &&
  "message" in value &&
  typeof (value as RpcErrorShape).code === "string" &&
  typeof (value as RpcErrorShape).message === "string";

export const parseJsonError = (str: string): RpcErrorShape | null => {
  try {
    const parsed = JSON.parse(str);
    return isRpcErrorShape(parsed) ? parsed : null;
  } catch {
    return null;
  }
};

export const createCallErrorFromShape = (shape: RpcErrorShape) =>
  createCallError(shape.code, shape.message, shape.details, shape.cause);

// =============================================================================
// Fiber Failure Extraction
// =============================================================================

const FiberFailureCauseId = Symbol.for("effect/Runtime/FiberFailure/Cause");

const extractFailuresFromCause = (cause: unknown): unknown[] => {
  if (!cause || typeof cause !== "object") return [];
  const c = cause as Record<string, unknown>;
  if (c._tag === "Fail") return [c.error];
  if (c._tag === "Die") return [c.defect];
  if (c._tag === "Sequential" || c._tag === "Parallel") {
    return [
      ...extractFailuresFromCause(c.left),
      ...extractFailuresFromCause(c.right),
    ];
  }
  return [];
};

// =============================================================================
// Error Parsing Options
// =============================================================================

export interface ErrorParserOptions {
  readonly parseJson?: boolean;
  readonly extractFiberFailure?: boolean;
  readonly unwrapNested?: boolean;
}

/**
 * Parse any error to Effect error with configurable options.
 */
export const parseToEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
  options: ErrorParserOptions = { parseJson: true },
): RpcEffectError => {
  if (isEffectRpcError(error)) return error;

  if (error instanceof Error && error.name === "AbortError") {
    return timeoutMs !== undefined
      ? createTimeoutError(path, timeoutMs)
      : createCancelledError(path);
  }

  if (options.extractFiberFailure) {
    if (
      typeof error === "object" &&
      error !== null &&
      FiberFailureCauseId in error
    ) {
      const cause = (error as Record<symbol, unknown>)[FiberFailureCauseId];
      if (cause && typeof cause === "object") {
        const failures = extractFailuresFromCause(cause);
        if (failures.length > 0) {
          return parseToEffectError(failures[0], path, timeoutMs, options);
        }
      }
    }
  }

  if (options.unwrapNested) {
    if (
      typeof error === "object" &&
      error !== null &&
      "error" in error &&
      (error as { error: unknown }).error !== undefined
    ) {
      return parseToEffectError(
        (error as { error: unknown }).error,
        path,
        timeoutMs,
        options,
      );
    }
  }

  if (isRpcErrorShape(error)) {
    return createCallErrorFromShape(error);
  }

  if (options.parseJson && typeof error === "string") {
    const parsed = parseJsonError(error);
    return parsed
      ? createCallErrorFromShape(parsed)
      : createCallError("UNKNOWN", error);
  }

  if (typeof error === "string") {
    return createCallError("UNKNOWN", error);
  }

  if (error instanceof Error) {
    if (options.parseJson) {
      const parsed = parseJsonError(error.message);
      if (parsed) return createCallErrorFromShape(parsed);
    }
    return createCallError("UNKNOWN", error.message, undefined, error.stack);
  }

  return createCallError("UNKNOWN", String(error));
};

/**
 * Parse transport error to Effect error.
 */
export const fromTransportError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError =>
  parseToEffectError(error, path, timeoutMs, { parseJson: true });

/**
 * Parse Effect fiber failure to Effect error.
 */
export const parseEffectError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcEffectError =>
  parseToEffectError(error, path, timeoutMs, {
    parseJson: true,
    extractFiberFailure: true,
    unwrapNested: true,
  });

/**
 * Parse any error to serializable RpcError.
 */
export const parseError = (
  error: unknown,
  path: string,
  timeoutMs?: number,
): RpcError => toRpcError(parseEffectError(error, path, timeoutMs));
