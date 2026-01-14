// =============================================================================
// RPC Logger Service
// =============================================================================

import { Effect, Layer } from "effect";
import type { RpcLogger } from "../core/types";

const noopLogger: RpcLogger = {
  debug: () => {},
  info: () => {},
  warn: () => {},
  error: () => {},
};

/** Console logger implementation */
export const consoleLogger: RpcLogger = {
  debug: (msg, data) => console.debug(`[RPC] ${msg}`, data ?? ""),
  info: (msg, data) => console.info(`[RPC] ${msg}`, data ?? ""),
  warn: (msg, data) => console.warn(`[RPC] ${msg}`, data ?? ""),
  error: (msg, data) => console.error(`[RPC] ${msg}`, data ?? ""),
};

/**
 * Logger service for debugging and monitoring.
 * Uses Effect.Service for idiomatic service definition with default.
 *
 * @example
 * ```ts
 * // Use default (noop logger)
 * Effect.provide(program, RpcLoggerService.Default)
 *
 * // Enable console logging
 * Effect.provide(program, RpcLoggerService.Console)
 *
 * // Custom logger
 * Effect.provide(program, RpcLoggerService.layer(myLogger))
 * ```
 */
export class RpcLoggerService extends Effect.Service<RpcLoggerService>()(
  "RpcLoggerService",
  {
    succeed: noopLogger,
  },
) {
  /** Create a layer with custom logger */
  static layer(logger: RpcLogger) {
    return Layer.succeed(RpcLoggerService, logger);
  }

  /** Layer with console logging enabled */
  static Console = Layer.succeed(RpcLoggerService, consoleLogger);
}
