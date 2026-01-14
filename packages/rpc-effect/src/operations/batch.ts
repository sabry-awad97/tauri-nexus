// =============================================================================
// Batch Operations
// =============================================================================

import { Effect } from "effect";
import type { RpcEffectError } from "../core/errors";
import {
  RpcTransportService,
  RpcLoggerService,
  type RpcServices,
} from "../services";
import { validatePath } from "../validation";
import { defaultParseError } from "./call";

// =============================================================================
// Types
// =============================================================================

export interface BatchRequestItem {
  readonly id: string;
  readonly path: string;
  readonly input: unknown;
}

export interface BatchResultItem<T = unknown> {
  readonly id: string;
  readonly data?: T;
  readonly error?: { code: string; message: string; details?: unknown };
}

export interface BatchRequest {
  readonly requests: readonly BatchRequestItem[];
}

export interface BatchResponse<T = unknown> {
  readonly results: readonly BatchResultItem<T>[];
}

// =============================================================================
// Batch Operations
// =============================================================================

/**
 * Validate batch requests (paths only).
 */
export const validateBatchRequests = (
  requests: readonly BatchRequestItem[],
): Effect.Effect<readonly BatchRequestItem[], RpcEffectError> =>
  Effect.gen(function* () {
    for (const req of requests) {
      yield* validatePath(req.path);
    }
    return requests;
  });

/**
 * Execute a batch of RPC calls using the transport's batch method.
 */
export const batchCall = <T = unknown>(
  requests: readonly BatchRequestItem[],
): Effect.Effect<BatchResponse<T>, RpcEffectError, RpcServices> =>
  Effect.gen(function* () {
    const transport = yield* RpcTransportService;
    const logger = yield* RpcLoggerService;

    for (const req of requests) {
      yield* validatePath(req.path);
    }

    logger.debug(`Executing batch with ${requests.length} requests`);

    const parseError = transport.parseError ?? defaultParseError;

    const response = yield* Effect.tryPromise({
      try: () => transport.callBatch<T>(requests),
      catch: (error) => parseError(error, "batch"),
    });

    return response as BatchResponse<T>;
  });
